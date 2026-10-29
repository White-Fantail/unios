#!/usr/bin/env python3
"""E2h QEMU host-side proof of firmware CR3 root identity, before guest deref.
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

ROOT_MASK = 0x000f_ffff_ffff_f000
TIME_LIMIT = 60
ERR = "E2H_HOST_WITNESS_REJECTED"

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
        ident=f"e2h-{self.counter}"
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

def run(argv):
    base=Path("out")
    qmp_path=base/"e2h-qmp.sock"
    gate_path=base/"e2h-com2.sock"
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
                qmp_socket.settimeout(8)
                gate_socket.settimeout(30)
                qmp=Qmp(qmp_socket)
                magic=b"\x7fUNI2H\x7f"
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
                (base/"e2h-host-root-proof.json").write_text(
                    json.dumps({"result":"PASS",**proof},indent=2)+"\n")
                (base/"e2h-host-hmp-transcript.txt").write_text(
                    "info registers before:\n"+regs1+"\n"+
                    "gva2gpa:\n"+translation+"\n"+
                    "xp:\n"+phys+"\nx:\n"+virt+"\n"+
                    "info registers after:\n"+regs2+"\n")
                if any(x in ("RESET","SHUTDOWN") for x in qmp.events):
                    raise RuntimeError("unexpected VM reset/shutdown before proof")
                # Authorize single guest gate, with CPU still paused.
                gate_socket.sendall(b"A")
                qmp.call("cont")
                remaining=max(1,int(timeout-time.monotonic()))
                rc=proc.wait(timeout=remaining)
                (base/"preflight-host-exit.txt").write_text(f"{rc}\n")
                if rc!=83:
                    raise RuntimeError(f"QEMU unexpected E2h diagnostic exit: {rc}")
                serial=(base/"preflight-serial.log").read_text(errors="replace")
                if "HALTED_PASS" in serial:
                    raise RuntimeError("E2h emitted forbidden kernel success marker")
                (base/"preflight-outcome.txt").write_text(
                    "E2H_QEMU_HOST_CR3_ROOT_CORROBORATED_ONLY "
                    "(NO EXITBS / NO NEW CR3 / NO CLEAN_PASS)\n")
                print(f"E2H_HOST_ROOT_QMP_PROOF PASS root={proof['root_pa']} "
                      f"qwords={proof['sampled_pml4_qwords']} exit={rc}")
                return 0
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=5)

if __name__=="__main__":
    if len(sys.argv)<3 or sys.argv[1]!="--":
        raise SystemExit("usage: e2h_root_host.py -- qemu-system-x86_64 [args...]")
    try:
        raise SystemExit(run(sys.argv[2:]))
    except (ValueError,RuntimeError,TimeoutError,OSError,subprocess.TimeoutExpired) as err:
        print(f"{ERR}: {err}",file=sys.stderr)
        raise SystemExit(1)
