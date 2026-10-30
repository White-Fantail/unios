"""E2r-C7: terminal-only clean kernel boot oracle, not a COM1-only test.

A correct ordered trace AND independently verified fifth QMP checkpoint
AND exact host process exit 65 must ALL be present. Never infer from
firmware output, guest-supplied flags, or diagnostics exit 133.
"""
ENTRY="UNIOS:L1:E2R_C6_HIGH_HALF_ENTRY"
SUCCESS="UNIOS:L1:HALTED_PASS"

def verify_trace(serial,exit_code,independent_qmp):
    if not isinstance(serial,str) or not independent_qmp or exit_code!=65:
        raise ValueError("clean boot requires exact exit 65 and independent kernel QMP")
    if serial.count(ENTRY)!=1 or serial.count(SUCCESS)!=1:
        raise ValueError("kernel entry/success COM1 markers missing or repeated")
    if serial.index(ENTRY)>serial.index(SUCCESS):
        raise ValueError("COM1 clean PASS occurred before entry")
    if "UNIOS:L1:E1_REJECTED" in serial or "UNIOS:L1:KERNEL_ENTRY_UNVERIFIED" in serial:
        raise ValueError("kernel rejection/unsupported preflight seen")
    return {"result":"PASS","kernel_entry_com1_marker":ENTRY,
            "kernel_success_com1_marker":SUCCESS,
            "ordered_com1":True,"exact_qemu_exit":65,
            "fifth_host_cpu_proof_required":True,
            "full_clean_boot":True}
