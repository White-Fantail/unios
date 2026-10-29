#!/usr/bin/env python3
"""E2q two pre-cut QMP witnesses and third host-owned live CR3/RIP/RSP CPU witness.
Only for pinned disposable LAB-H1; never a portable firmware guarantee.
QMP HMP is an intentionally version-dependent QEMU 8.x interface.
"""
import json
import os
import re
import socket
import subprocess
import sys
import time
from pathlib import Path

from e2i_table_walk import walk_tables, qmp_read_table
from e2j_targeted import MARKER, parse_manifest, verify_targets
from e2o_new_tree import MARKER as PRIVATE_MARKER
from e2q_new_tree import PACKET,decode_manifest,verify_new,MARKER as SWITCH_MARKER,parse_switched_regs

ROOT_MASK = 0x000f_ffff_ffff_f000
TIME_LIMIT = 120
ERR = "E2Q_HOST_ACTIVE_CR3_REJECTED"

def parse_cr3(registers):
    found = re.search(r"\bCR3=([0-9a-fA-F]{8,16})\b", registers, re.I)
    if not found:
        raise ValueError("QMP registers did not expose CR3")
    raw = int(found.group(1), 16)
    root = raw & ROOT_MASK
    if root < 0x100000 or root >= 0x100000000 or raw & 0xfff & ~0x18:
        raise ValueError("bad QMP CR3 root")
    return raw, root

def parse_translation(text, root):
    # QEMU 8.x HMP gva2gpa prints ONLY "gpa: 0x...", not the GVA.
    # Accept two pinned documented reply shapes; reject any extra lines.
    s=text.strip()
    one=re.fullmatch(r"(?i)gpa:\s*0x([0-9a-f]+)",s)
    two=re.fullmatch(
        r"(?i)gva\s+0x([0-9a-f]+)\s*->\s*gpa\s+0x([0-9a-f]+)",s)
    if one and int(one.group(1),16)==root:
        return True
    if two and int(two.group(1),16)==root and int(two.group(2),16)==root:
        return True
    raise ValueError(f"host QMP root mapping is not identity: {s[:300]!r}")

def parse_words(text, address, n=8):
    words=[]
    positions=[]
    for line in text.splitlines():
        if ":" not in line:
            continue
        left,right=line.split(":",1)
        try:
            loc=int(left.strip(),16)
        except ValueError:
            continue
        word_hex=re.findall(r"0x([0-9a-fA-F]{1,16})\b",right)
        if not word_hex:
            continue
        positions.append(loc)
        words.extend(int(v,16) for v in word_hex)
    if not positions or positions[0] != address or len(words)!=n:
        raise ValueError("physical or virtual QEMU root bytes unreadable")
    if not any(v != 0 and v != 0xffffffffffffffff for v in words):
        raise ValueError("QEMU root PML4 entries appear empty/invalid")
    return words

def verify_witness(reg1, translate, physical, virtual, reg2):
    a,root=parse_cr3(reg1)
    b,root2=parse_cr3(reg2)
    if a!=b or root!=root2:
        raise ValueError("guest CR3 changed across paused verification")
    parse_translation(translate,root)
    pv=parse_words(physical,root)
    vv=parse_words(virtual,root)
    if pv!=vv:
        raise ValueError("QEMU physical and virtual root reads differ")
    return {"cr3":f"0x{a:x}", "root_pa":f"0x{root:x}",
            "sampled_pml4_qwords":len(pv),"root_phys_virtual_equal":True,
            "witness":"QMP halted CPU0; gva2gpa and xp/x checked"}

class Qmp:
    def __init__(self,sock):
        self.sock=sock
        self.f=sock.makefile("rb")
        self.counter=0
        self.events=[]
        greeting=self.read()
        if "QMP" not in greeting:
            raise RuntimeError("QMP greeting unavailable")
        self.call("qmp_capabilities")
    def read(self):
        line=self.f.readline()
        if not line:
            raise RuntimeError("QMP control connection lost")
        return json.loads(line)
    def call(self,method,arguments=None):
        self.counter+=1
        ident=f"e2i-{self.counter}"
        data={"execute":method,"id":ident}
        if arguments is not None: data["arguments"]=arguments
        self.sock.sendall((json.dumps(data)+"\n").encode())
        while True:
            result=self.read()
            if "event" in result:
                self.events.append(result["event"])
                continue
            if result.get("id")!=ident:
                raise RuntimeError("unexpected QMP response id")
            if "error" in result:
                raise RuntimeError(f"QMP failure: {result['error']}")
            return result["return"]
    def hmp(self,line):
        answer=self.call("human-monitor-command",{"command-line":line,"cpu-index":0})
        if not isinstance(answer,str):raise RuntimeError("bad HMP result")
        return answer

def connect_unix(path, process, deadline):
    while time.monotonic()<deadline:
        if process.poll() is not None:
            raise RuntimeError(f"QEMU exited before host proof: {process.returncode}")
        try:
            s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
            s.settimeout(3)
            s.connect(str(path))
            return s
        except (OSError,TimeoutError):
            s.close()
            time.sleep(0.05)
    raise TimeoutError(f"QEMU {path} control socket not ready")

def expected_host_exit(mode):
    choices={"preflight":125,"ud":103,"ist":105,"df":109,"exitbs":115,"cr3":121}
    if mode not in choices:
        raise ValueError("unknown QEMU handler smoke mode")
    return choices[mode]

def run(argv):
    mode=os.environ.get("UNIOS_E2Q_SMOKE","preflight")
    expected_exit=expected_host_exit(mode)
    base=Path("out")
    qmp_path=base/"e2j-qmp.sock"
    gate_path=base/"e2j-com2.sock"
    for path in (qmp_path,gate_path):
        try:path.unlink()
        except FileNotFoundError:pass
    timeout=time.monotonic()+TIME_LIMIT
    cmd=argv+[
        "-qmp",f"unix:{qmp_path},server=on,wait=off",
        "-serial",f"unix:{gate_path},server=on,wait=off"
    ]
    with (base/"preflight-qemu-stdout.log").open("wb") as stdout, \
         (base/"preflight-qemu-stderr.log").open("wb") as stderr:
        proc=subprocess.Popen(cmd,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL)
        try:
            with connect_unix(qmp_path,proc,timeout) as qmp_socket, \
                 connect_unix(gate_path,proc,timeout) as gate_socket:
                qmp_socket.settimeout(12)
                gate_socket.settimeout(30)
                qmp=Qmp(qmp_socket)
                magic=MARKER
                collected=bytearray()
                while magic not in collected:
                    if time.monotonic()>timeout:
                        raise TimeoutError("guest did not emit unique barrier marker")
                    fragment=gate_socket.recv(1024)
                    if not fragment:
                        raise ValueError("COM2 socket closed before barrier")
                    collected.extend(fragment)
                    if len(collected)>128*1024:
                        raise ValueError("excess COM2 noise before guest barrier")
                suffix=collected.split(magic,1)[1]
                while len(suffix)<1:
                    fragment=gate_socket.recv(1024)
                    if not fragment: raise ValueError("missing manifest count")
                    suffix.extend(fragment)
                count=suffix[0]
                if not 8<=count<=16: raise ValueError("bad manifest record count")
                expected=1+13*count
                while len(suffix)<expected:
                    if time.monotonic()>timeout: raise TimeoutError("manifest timeout")
                    fragment=gate_socket.recv(1024)
                    if not fragment: raise ValueError("guest COM2 closed during manifest")
                    suffix.extend(fragment)
                if len(suffix)!=expected: raise ValueError("manifest trailing bytes")
                records=parse_manifest(bytes(suffix))
                # Pause BEFORE reading registers/translation/memory; proof is
                # independent of any physical-address value printed by guest.
                qmp.call("stop")
                status=qmp.call("query-status")
                if status.get("running") is not False:
                    raise RuntimeError("QEMU vCPU failed to pause")
                regs1=qmp.hmp("info registers")
                _,root=parse_cr3(regs1)
                translation=qmp.hmp(f"gva2gpa 0x{root:x}")
                phys=qmp.hmp(f"xp /8gx 0x{root:x}")
                virt=qmp.hmp(f"x /8gx 0x{root:x}")
                regs2=qmp.hmp("info registers")
                proof=verify_witness(regs1,translation,phys,virt,regs2)
                full=walk_tables(root,lambda pa:qmp_read_table(qmp,pa),max_depth=1)
                regs3=qmp.hmp("info registers")
                _,later=parse_cr3(regs3)
                if later!=root:
                    raise RuntimeError("CR3 changed during full-page-table witness")
                proof.update(full)
                targeted=verify_targets(root,records,qmp)
                proof.update(targeted)
                proof["new_private_cr3_host_independently_audited"]=False
                proof["guest_raw_cut_audit"]="inferred_from_distinct_guest_preflight_exit_only"
                (base/"e2q-old-root-precheck.json").write_text(
                    json.dumps({"result":"PASS",**proof},indent=2)+"\n")
                (base/"e2q-host-hmp-root-sample.txt").write_text(
                    "info registers before:\n"+regs1+"\n"+
                    "gva2gpa:\n"+translation+"\n"+
                    "xp:\n"+phys+"\nx:\n"+virt+"\n"+
                    "info registers after:\n"+regs2+"\n")
                if any(x in ("RESET","SHUTDOWN") for x in qmp.events):
                    raise RuntimeError("unexpected VM reset/shutdown before proof")
                # Authorize single guest gate, with CPU still paused.
                gate_socket.sendall(b"J")
                qmp.call("cont")
                # A second, independent NEW table witness must follow the
                # guest's completed raw-cut audit. Do not infer success
                # from a UART marker; halt the vCPU, then independently
                # read every physical PTE frame via paused QMP.
                collected=bytearray()
                while PRIVATE_MARKER not in collected:
                    if time.monotonic()>timeout:
                        raise TimeoutError("guest omitted new-PML4 host checkpoint")
                    fragment=gate_socket.recv(1024)
                    if not fragment:
                        raise ValueError("COM2 closed before private-table manifest")
                    collected.extend(fragment)
                    if len(collected)>128*1024:
                        raise ValueError("excess COM2 traffic before new-root checkpoint")
                second=bytearray(collected.split(PRIVATE_MARKER,1)[1])
                while len(second)<PACKET:
                    if time.monotonic()>timeout:
                        raise TimeoutError("new root manifest timed out")
                    fragment=gate_socket.recv(1024)
                    if not fragment:
                        raise ValueError("truncated private PML4 manifest")
                    second.extend(fragment)
                if len(second)!=PACKET:
                    raise ValueError("trailing bytes in private PML4 manifest")
                new_root,used,new_targets=decode_manifest(bytes(second))
                qmp.call("stop")
                if qmp.call("query-status").get("running") is not False:
                    raise RuntimeError("QEMU failed to pause for private PML4 proof")
                regs4=qmp.hmp("info registers")
                _,old_again=parse_cr3(regs4)
                if old_again!=root:
                    raise RuntimeError("private root became active or old CR3 changed")
                old_cache={}
                def old_read(pa):
                    if pa not in old_cache:
                        if len(old_cache)>=64:
                            raise ValueError("old-CR3 executable witness cap")
                        old_cache[pa]=tuple(qmp_read_table(qmp,pa))
                    return old_cache[pa]
                new_proof=verify_new(
                    new_root,used,new_targets,records,root,
                    lambda pa:qmp_read_table(qmp,pa),old_read)
                regs5=qmp.hmp("info registers")
                _,old_final=parse_cr3(regs5)
                if old_final!=root:
                    raise RuntimeError("old firmware CR3 changed during new-PTE audit")
                proof.update(new_proof)
                proof["guest_raw_cut_audit"]="completed_before_second_paused_checkpoint"
                (base/"e2q-host-targeted-proof.json").write_text(
                    json.dumps({"result":"PASS",**proof},indent=2)+"\n")
                (base/"e2q-new-root-host-proof.json").write_text(
                    json.dumps({"result":"PASS","old_root":f"0x{root:x}",
                                **new_proof},indent=2)+"\n")
                if any(x in ("RESET","SHUTDOWN") for x in qmp.events):
                    raise RuntimeError("unexpected reset before second authorization")
                # Only the successful independent physical-frame audit is
                # allowed to unlock the guest; this is not a CR3 switch.
                gate_socket.sendall(b"O")
                qmp.call("cont")
                if mode=="cr3":
                    # CPU now calls ExitBootServices, sets bridge RSP,
                    # executes an actual MOV CR3, and sends the UNIQUE
                    # third COM2 marker from RX code under new CR3.
                    third=bytearray()
                    while SWITCH_MARKER not in third:
                        if time.monotonic()>timeout:
                            raise TimeoutError("no post-CR3 terminal COM2 checkpoint")
                        fragment=gate_socket.recv(512)
                        if not fragment:
                            raise ValueError("COM2 lost after terminal CR3 switch")
                        third.extend(fragment)
                        if len(third)>8192:
                            raise ValueError("post-switch COM2 marker missing")
                    if third.split(SWITCH_MARKER,1)[1]:
                        raise ValueError("extraneous data after third marker")
                    qmp.call("stop")
                    if qmp.call("query-status").get("running") is not False:
                        raise RuntimeError("QEMU vCPU was not stopped post-CR3")
                    cpu=qmp.hmp("info registers")
                    bridge=next(r for r in records if r["id"]==3)
                    live=parse_switched_regs(
                        cpu,new_root,
                        int(new_proof["cr3_terminal_code_begin"],16),
                        int(new_proof["cr3_terminal_code_end"],16),
                        bridge["base"],bridge["end"])
                    rip=int(live["cpu_live_rip"],16)
                    from e2i_table_walk import PAGE
                    if verify_new is None:raise RuntimeError("missing new host walker")
                    # Separate check of the actual CPU linear instruction
                    # address (distinct from guest-provided begin/end).
                    from e2o_new_tree import walk_new as walk_active
                    pc_mapping=walk_active(
                        new_root,used,rip,lambda pa:qmp_read_table(qmp,pa))
                    if pc_mapping!=(rip,1):
                        raise ValueError("actual post-switch RIP is not private RX identity")
                    # Confirm processor's active translation, not merely
                    # the raw PTE bytes, using QEMU's own GVA translation.
                    parse_translation(qmp.hmp(f"gva2gpa 0x{rip:x}"),rip)
                    cpu_after=qmp.hmp("info registers")
                    if parse_cr3(cpu_after)[1]!=new_root:
                        raise RuntimeError("CPU CR3 changed during third host proof")
                    proof.update(live)
                    (base/"e2q-live-cr3-proof.json").write_text(
                        json.dumps({"result":"PASS",**live,
                                    "pc_mapping_identity_rx":True,
                                    "qmp_gva2gpa_identity":True,
                                    "efi_boot_services_exited":"inferred_from_terminal_marker_after_success_branch"},
                                   indent=2)+"\n")
                    (base/"e2q-post-cr3-cpu-registers.txt").write_text(cpu)
                    if any(x in ("RESET","SHUTDOWN") for x in qmp.events):
                        raise RuntimeError("VM shutdown before terminal new-CR3 verification")
                    gate_socket.sendall(b"Q")
                    qmp.call("cont")
                remaining=max(1,int(timeout-time.monotonic()))
                rc=proc.wait(timeout=remaining)
                (base/"preflight-host-exit.txt").write_text(f"{rc}\n")
                if rc!=expected_exit:
                    raise RuntimeError(f"QEMU unexpected E2q diagnostic exit: {rc}, expected={expected_exit} mode={mode}")
                serial=(base/"preflight-serial.log").read_text(errors="replace")
                if "HALTED_PASS" in serial:
                    raise RuntimeError("E2q emitted forbidden kernel success marker")
                (base/"preflight-outcome.txt").write_text(
                    f"E2Q_QEMU_ACTUAL_CR3_HOST_WITNESS_{mode.upper()}_SMOKE_ONLY "
                    "(NO EXITBS / NO NEW CR3 / NO CLEAN_PASS)\n")
                print(f"E2Q_QMP_OLD_NEW_AND_ACTIVE_CR3_WITNESS PASS mode={mode} root={proof['root_pa']} "
                      f"targets={proof['sampled_reservation_pages']} "
                      f"tables={proof['fully_compared_table_pages']} "
                      f"leaf_sizes={proof['sampled_leaf_sizes']} "
                      f"new_root={new_proof['new_root_pa']} "
                      f"new_pages={new_proof['new_table_pages_compared']} "
                      f"new_targets={new_proof['new_target_count']} "
                      f"new_guards={new_proof['new_unmapped_guards']} exit={rc}")
                return 0
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=5)

if __name__=="__main__":
    if len(sys.argv)<3 or sys.argv[1]!="--":
        raise SystemExit("usage: e2q_host.py -- qemu-system-x86_64 [args...]")
    try:
        raise SystemExit(run(sys.argv[2:]))
    except (ValueError,RuntimeError,TimeoutError,OSError,subprocess.TimeoutExpired) as err:
        print(f"{ERR}: {err}",file=sys.stderr)
        raise SystemExit(1)
