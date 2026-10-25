# UniOS RFC-003 — IPC Wire Protocol & Synchronization

> **Status:** Proposed — Specification v0.2 (재검토 개정·미승인·미검증)
> **Date:** 2026-10-24 (NZDT)
> **Scope:** IPC 메시지, Kernel Envelope/Service Wire, Channel/ReplyToken/Event/WaitSet, 핸들 MOVE, 원자성, 시간 제한, 취소, backpressure, 장애 복구와 검증
> **Baseline:** [Architecture v0.2](../architecture/unios-architecture-v0.2.md); [RFC-001 v0.2](rfc-001-kernel-boundary-and-tcb.md); [RFC-002 v0.2](rfc-002-object-capability-lifetime-delegation-revocation.md)
> **Limitation:** 설계·연구 전용. 커널·SDK·서비스 구현, 수치 ABI 고정, 성능 측정, 형식 검증 또는 구현 승인 없음.

## 0. Executive recommendation

**권장 연구 기준선 IPC-A:** bounded asynchronous Channel (**E1 단일 소유 Endpoint 연구안**) + one-shot ReplyToken + Event/WaitSet (**W1/W2 관찰 계약 비교**) + optional shared MemoryObject. 커널은 메시지 전달과 보호·소유권·회계·대기/깨우기를 수행하고, 파일/네트워크/GUI/Windows/Linux/Darwin 의미론이나 복잡한 RPC 파서는 user-space SABI 서비스에 배치한다.

**v0.2 변경:** Endpoint 소유권, 수신 커밋 전 데이터 공개, ReplyToken과 Supervisor 인스턴스의 구별, WaitSet observer 수명, backpressure의 busy retry 및 Channel 순환을 강화한다.

이 모델이 최적이라고 **확정하지 않는다**. IPC-B는 seL4 유사 rendezvous-first synchronous endpoint, IPC-C는 completion-queue 중심 asynchronous RPC/port를 비교 기준선으로 유지한다. IPC-A의 커널 queue/reply/receipt/WaitSet 객체 수 증가와 동시성 리스크를 모델 검사·향후 성능 비교로 정당화할 수 없으면 단순화하거나 IPC-B/C를 다시 심사한다.

**절대 혼동 금지:**
1. Kernel message COMMITTED는 receiver가 메시지를 읽거나 서비스 작업을 완료했다는 뜻이 아니다.
2. Wait timeout은 대기 종료이지 원격 file write·DMA 작업의 성공적 취소가 아니다.
3. Capability MOVE와 한 Task가 소유한 정수 handle의 복사는 다르다. 커널의 provenance·rights·scope·sponsor가 이동해야 한다.
4. 이벤트 통지는 메시지 카운터가 아니며 readiness는 재검사 가능한 힌트다.
5. Kernel Envelope는 SABI의 versioned IDL/message format과 다르고 Linux/NT/Darwin syscall 해석을 담지 않는다.

### 근거와 설계 관계

- **D5/ADR-002, RFC-001:** kernel은 최소 보호/스케줄링/IPC primitive 담당; service policy/user identity/guest semantics는 kernel 바깥.
- **RFC-002:** Task-local handle, typed rights, MOVE의 단일 commit, NOT_COMMITTED/COMMITTED/UNKNOWN, lineage anchor, sponsor accounting, B/R1/R2-F/R2-O revocation 상태를 유지.
- **D7/ADR-009:** 일반 사용자 보안 위협 모델을 처음부터 고려, 보증 수준은 단계별 검증.
- **RFC-004/006:** Pager boot/resource recovery deadlock을 kernel fast IPC보다 먼저 해소.
- **RFC-005:** scheduling-context donation, priority propagation과 실시간 성질은 아직 미승인.

## 1. 목표, 비목표, 위협

### 1.1 Goals

- 소형 native CLI/GUI와 user-space services 사이의 권한 검증형 메시지 전달.
- Multi-core sender/receiver, object close, revoke, OOM, server crash, timeout, cancellation 경합에 대한 단일 선형화 지점 및 정의된 결과.
- 한 요청의 bytes + Capability grants 전송은 전부 커밋되거나 하나도 커밋되지 않음.
- 비신뢰 송신자의 큐/handle/reply/wait 등록 폭증 제한; Pager/boot emergency pool 분리.
- 서비스 재시작 시 오래된 reply/cap/grant가 새 서비스 incarnation으로 자동 승격되지 않도록 강제.
- Kernel data plane·SABI protocol·SDK와 ABI versioning, compatibility personality의 경계 분명히 정의.

### 1.2 Non-goals

- POSIX sockets, Linux futex/epoll, Windows ALPC, Mach port 의미론을 Ring 0에 전부 직접 포함.
- 크로스 머신 distributed RPC, persistent queue, 모든 요청의 exactly-once external effects.
- final syscall numbers, raw struct layout, endian, inline size/handle count, stable v1 ABI.
- 성능·장치 DMA·하드웨어 검증·formal proofs 달성 주장.

### 1.3 Threat model

비신뢰 앱, 공격자가 장악한 broker/driver/service, 악의적 data length/handle descriptor, duplicate handle array, usercopy TOCTOU, 타 Task identity 위장, sender flood, peer kill/restart, stale reply ID, IPC queue circular refs, timeout/revoke race, SMP TLB/wait ordering, Pager page fault during IPC, priority donation 공격. 물리적 악성 펌웨어/CPU 결함은 RFC-001에 정한 외부 신뢰 가정으로 따로 추적한다.

## 2. 공식 1차 설계 사례

| 원전 | 실제 공개 설계 내용 | UniOS 설계 교훈 |
|---|---|---|
| seL4 IPC [S1] | 동기 endpoint, Send/Recv/Call/Reply, cap transfer, badge와 small-message fastpath | rendezvous와 one-shot reply의 작은 TCB·성능 기준 |
| seL4 MCS [S2] | scheduling context, passive server, reply object, 시간 예산 기부 | IPC 답장과 CPU budget 반환·timeout은 결합될 수 있으나 RFC-005 합동 검증 필요 |
| seL4 Notification [S3] | signal coalescing, OR된 badge, shared memory notification | 알림은 정확한 메시지 수가 아니라 상태·힌트 |
| Zircon Channel [S4,S5] | 양방향 queue의 bytes/handles 이동; **Channel Endpoint는 복제할 수 없는 단일 owner handle** | UniOS의 다중 Task 동일 Endpoint 소유를 E2 별도 연구로 분리 |
| Zircon Channel Call [S6] | transaction ID 응답 매칭, timeouts/late reply 관련 주의 | 숫자 ID 재사용만으로 잘못된 늦은 응답을 막지 못함 |
| Zircon Wait/Signals [S7,S8] | 비동기 observer가 packet enqueue 후 종료되는 **one-shot zx_object_wait_async** | UniOS의 persistent-level WaitSet과 동일하지 않으므로 W1/W2 비교 |
| Fuchsia FIDL [S9,S10] | service ordinals/version/unknown interaction/transaction header | SABI IDL·wire format은 kernel native syscall ABI와 분리 |

**참고 사실과 UniOS 제안을 구별한다.** 특히 seL4 rendezvous와 Zircon 기본 write의 handle consume-on-error는 UniOS의 proposed IPC-A와 동일하다고 가정하지 않는다.

## 3. KABI transport 대 SABI application wire

### 3.1 Kernel-owned message envelope: 논리 구조 (wire layout 아님)

| 논리 필드 | 제공 주체 | 커널 처리 | 신뢰 범위 |
|---|---|---|---|
| destination endpoint handle | sender | task-local type + SEND rights, peer state, revocation epoch 확인 | kernel object |
| bytes pointer + length / inline data | sender | upper bound, overflow, copyin, fault discipline | 데이터 내용은 비신뢰 |
| transfer descriptors | sender | move-only, scoped handle rights, lineage, charge 검증 | handle 정수는 Task-local |
| optional kernel ReplyToken | kernel on authorized call | one-shot, caller+request+instance generation binding | reply 권한은 user bytes로 위조 불가 |
| trusted badge/peer label 후보 | kernel-minted capability context | scope 검증, 앱 ID 문자열과 구별 | origin 검증이지만 user consent는 아님 |
| queue order/commit metadata | kernel | FIFO per direction, charge, state/sequence | kernel 내부 ID는 접근 credential 아님 |
| service header | user-space SABI | kernel은 임의 method/field 의미 해석 안 함 | 요청 ID/버전/인증은 해당 서비스 검증 |

**명시적으로 동결하지 않음:** syscall descriptor의 실제 bit layout, signed/unsigned size, byte order, alignment, raw structs, count limit, header offset, kernel return register encoding. RFC-009/018에서 향후 versioned KABI/compat rules와 함께 결정.

### 3.2 User-space SABI envelope / IDL

Service의 versioned protocol은 필요한 경우 method_id, protocol_id/version, request_id, service incarnation, typed payload, return/error, timeout/cancel/idempotency policy 등을 정의한다. 이 필드가 **전부 kernel message header에 들어가야 한다는 의미는 아니다.**

- strict required method와 unknown optional 필드는 다르게 처리; unknown enum/union, 중복·중첩 필드, 길이, UTF-8, buffer bounds, handle-role 검증.
- SABI name, PID, guest UID/GID, registry URL은 커널 권한 자격증명이 아니다. Service discovery ≠ capability grant.
- API negotiation/버전 협상은 서비스가 수행하되 커널 수신 자료의 권한 검증을 대체하지 못한다.
- 서비스 서명 검증도 해당 서비스 파서의 memory-safety 또는 논리 정확성을 보증하지 않는다.

## 4. 객체와 권한

| Object candidate | type-specific rights | 안 되는 암묵적 효과 |
|---|---|---|
| ChannelEndpoint (**E1-only profile**) | SEND, RECEIVE, WAIT, TRANSFER, QUERY_BASIC (**DERIVE/DUPLICATE 금지**) | 상대 Task의 메모리·debug state 접근; E2 다중 소유권을 암묵적으로 부여하지 않음 |
| ReplyToken | REPLY_ONCE, 조건부 WAIT_STATUS / 제한된 WORKER_TRANSFER | arbitrary Task reply / token re-use |
| Event | SIGNAL, WAIT | 정확한 이벤트 개수 전달 보증 |
| WaitSet | REGISTER, UNREGISTER, WAIT | 관찰 대상 Capability 소유권 상승 |
| WaitRegistration | REARM/REMOVE (내부 token 후보) | revoked object에 대한 무단 callback |
| MemoryObject | typed MAP/READ/WRITE/EXECUTE scope | Channel SEND만으로 무단 map |
| ResourceBudget | ALLOCATE, OBSERVE, DELEGATE 범위 | 크기만으로 다른 Task 자원 취득 |

**2026-10-25 Cross-RFC 설계 문구 정정 (Proposed 유지):** 위 권한 표는 RFC-003 §4 본문 및 IPC-D13과 모순되었던 E1 Endpoint의 DERIVE 열거를 제거했다. E1 Endpoint 하나는 동시에 유효 소유 Task 한 곳에만 속하고, 별도 클라이언트 연결은 독립 Channel 쌍/명시적 dispatcher를 통해 구성한다. E2 다중-Task 공유는 별도 미결 연구다. 권한, 실제 caller, MOVE/ReceiveClaim/UNKNOWN의 통합 계약은 [CAPIPC 공동 설계안 v0.1](../contracts/capability-ipc-sabi-integration-contract-v0.1.md) 참조. **코드/와이어/숫자 ABI, 선택된 E1 알고리즘의 증명이나 Accepted 변경이 아니다.**

**초기 권고:** 메시지 내 Capability 이동은 MOVE-only. COPY하려면 RFC-002 DERIVE로 새 grant를 만든 뒤 MOVE. **E1 ChannelEndpoint 자체는 DERIVE/DUPLICATE 불가 타입**으로 제안. 원본 권한·계보·revoke anchor는 보존. 별도 DUPLICATE_IN_SEND는 성능상 필요성이 입증되기 전까지 보류. 이 역시 확정 API가 아니라 비교 기준.

## 5. Bounded Channel & ordering

- 두 Endpoint를 가진 full-duplex Channel의 방향별 유한 큐 후보. **E1 프로파일에서는 Endpoint당 유효 소유 handle이 하나**이고, 동일 Task의 스레드끼리 공유 가능하지만 Task 간에는 MOVE로만 이전한다. E2 다중 Task 소유는 별도 제안. in-transit Endpoint와 이미 허가된 invocation reference는 handle table 소유권과 수명이 다르다.
- 한 방향의 성공적으로 커밋된 메시지에 대해 FIFO total order. Multi-core concurrent sender는 서로 경쟁하며 **커밋 순서**가 순서를 정의한다. 반대 방향과 다른 Channel 간 global ordering 없음.
- 한 queue record는 bytes + move된 kernel caps metadata + sponsor charge를 하나의 단위로 취급. Receiver는 모든 handles가 설치되거나 아무것도 설치되지 않는다.
- Readiness는 같은 queue에 대한 다른 receiver가 먼저 소모하면 사라질 수 있는 상태. IPC-A는 Peek 이후 레이스가 가능하므로 repeated receive에 WOULD_BLOCK가 정상.
- Queue messages는 **비영속**. 전원 재시작, 서비스 crash, endpoint Abort는 재전송·durable commit을 보증하지 않는다.
- 임의 peer-name lookup/registry에서의 암묵적 cap 발급과 hidden inheritance를 금지한다.
- **송신에 이용하는 Endpoint 자신을 동일 메시지의 MOVE 목록에 넣는 것을 금지**한다. Cross-channel cycle은 bounded iterative reclaim 또는 초기 기능 제한 필요.
- Endpoint MOVE와 이미 시작된 send/receive/close는 선형화/인용 참조 수명 계약이 있어야 한다.

### 5.1 송신 전이

~~~text
Validate endpoint SEND, MOVE sources, scope, revocation barrier
  -> Prepare copyin and bounded quota reservations
  -> One Commit: queue append AND sender handles removed
  -> Report: NOT_COMMITTED / COMMITTED / UNKNOWN
  -> Receiver separate commit: dequeue AND install all caps
  -> Service processes SABI request (not part of kernel commit)
~~~

**NOT_COMMITTED:** 실제 큐 등록 이전에 실패. 송신자의 모든 source handles를 보존하며 임시 charge 원복. **COMMITTED:** 큐에 온전한 record가 게시되고 sender는 이동된 핸들을 소유하지 않음. **UNKNOWN:** 송신자에게 authoritative result 전달이 안 됨. Kernel 내부 커밋의 참값은 존재하지만 호출자가 모를 수 있음. UNKNOWN에서 blind retry 금지.

### 5.2 Commit → report 결과 유실의 P0 난제

User pointer fault, 스레드 종료, signal/interrupt와 커널 반환을 혼동하면 “에러를 받았으니 handle이 살아 있다”라는 잘못된 전제를 만들 수 있다.

| 설계 대안 | 방법 | 위험 |
|---|---|---|
| **T1 protected completion receipt** (우선 연구 가설) | commit 전에 커널이 Task-scoped operation record/ticket를 예약·회계하고 상태 확인 가능하게 함 | ticket 발급 자체 결과 유실, orphan/GC, 권한 범위, retention |
| T2 preallocated immutable result slot | 커밋 전 결과가 안전하게 기록될 공간을 예약하여 원자적 결과 전달 시도 | user page remap/fault, pin 비용, Pager 교착 |
| T3 fail-closed ambiguous caller | 생존 Task의 불확정 상황에서 해당 context 격리·종료 및 supervisor reconciliation | 낮은 UX, 외부 I/O 처리 상태는 여전히 불확정 |

T1 역시 ticket의 ID가 syscall 반환 시 유실되는 경우를 해결해야 한다. Task namespace의 제한된 pending-operation enumeration, prior reserved operation slot 등이 후보지만 커널 권한·메타데이터 누출 위험이 크다. **RFC-003 v0.2도 어느 알고리즘도 이미 안전하다고 주장하지 않는다.**

- 커밋 전 결과/보조 커널 메타데이터는 quota 예약되어야 한다. 실패 후 임시 메타데이터가 살아남지 않음.
- 커밋 후 대기 중인 결과를 무조건 rollback하지 않는다. 명시적 status reconcile 또는 fail-closed 상태로 정리한다.
- Service-level request_id와 kernel operation ticket은 다른 역할. 이미 수행한 외부 I/O에 대한 재시도는 SABI idempotency 정책을 따름.
- Process death 후 supervisor가 ticket을 관측할 필요와 접근 범위도 별도 승인.

### 5.3 실패 시 handle 보존

Zircon의 기본 zx_channel_write는 성공·실패 모두 전달 handle을 소비한다고 문서화한다 [S5]. zx_channel_write_etc는 MOVE/DUPLICATE 등의 분기를 제공한다 [S11]. UniOS는 **커밋되지 않은 MOVE에서는 원본 전부 보존**, 커밋 후에는 보존하지 않는 계약을 제안한다. 외부 OS의 semantics를 UniOS에 전이하지 않는다.

## 6. Receive, copyout & atomic install

송신 commit과 수신 commit은 다르다. 가장 중요한 위험은 **handle table에 cap을 설치하는 순간과 user buffer에 수신 결과를 복사하는 순간을 단일 원자 연산으로 묶기 어렵다**는 것이다.

### 6.1 Receive candidate stages

1. Endpoint RECEIVE/WAIT rights, message length/handle count, revoked status 확인.
2. Receiver handle-slot budget 예약. Kernel staging 또는 fault-safe user buffer copyout 준비.
3. Atomic receive commit: queue record를 제거하는 동시에 모든 cap이 Task table로 이동·설치되거나 **둘 다 수행하지 않음**.
4. Return/report: client가 commit 결과를 알 수 있도록 protected result/escrow/retry 계약. 단순 user pointer 검증만으로 write가 안전하다고 하지 않음.
5. 실패 시: 부분 copyout bytes가 user buffer를 변경할 가능성은 별도 기록하되, 부분 cap install이나 grant duplication을 허용하지 않음.

### 6.2 비교할 receive 전략

- **RCV-A ReceiveEscrow 후보:** 한 Task의 독점 Claim 및 kernel-managed staging/receipt 후보. 사용자 버퍼 공개 이전 Claim/권한 인가를 별도로 정의하고 Task death/revoke/timeout과 copyout 실패의 상태 전이를 검증. Kernel escrow retention/quota 증가는 별도 P0.
- **RCV-B copy-before-commit:** **hostile receiver 모델에서 기본 기각.** 사용자 메모리에 payload를 공개하면 receive commit이 실패해도 회수할 수 없다. 다른 receiver가 같은 message를 소비할 수 있다. 독점 Claim과 권한 인가, 공개 자체의 커밋 의미, 실패 시 노출 범위를 증명하지 않는 한 채택 금지.
- **RCV-C register-only small IPC:** 인라인 메시지를 register-return에 제한해 copyout 혼란 감소, bulk는 MemoryObject 경로에서 처리. 범용 데스크톱 프로토콜에 불편할 수 있음.

**P0 보류:** RCV-A/B/C의 정확한 선택, in-kernel lock order, usercopy fault 중 Pager 재진입, 다중 수신자 레이스. Invalid user pointer 때문에 커널 패닉/권한 상승/자기 pager 교착이 발생하지 않아야 한다.

**2026-10-25 CAPIPC 모델 v0.2 대조 (Proposed, 검증되지 않음):** [통합 모델 v0.2](../models/capipc-send-receive-revoke-state-model-v0.2.md)은 K-C Claim, K-Vm (민감 metadata 관측 가능성), K-Vp (payload 노출 시도 전 단조 표시), K-I (cap-table all-or-none install), 사용자-visible Handle token 출력, K-U (T2에서 독립 승인된 권한 활성화)를 분리한다. 특히 §6.1의 'cap 설치와 dequeue의 원자성'은 **사용자 메모리에 반환되는 핸들 번호 전체의 원자성**이나 **동시 실행 중인 수신 Task 스레드가 권한을 언제 사용할 수 있는가**를 자동 증명하지 않는다. 토큰 일부 출력 뒤 fault, 먼저 설치된 권한의 결과 분실 및 public-token ABA/고아 권한 회수는 P0 미해결이다. **RCV-T1 연구 우선, RCV-T2는 별도 신뢰된 검증자와 K-U 당시 권한/철회/정책 재검증이 있는 특수 프로파일 후보**이며, RFC-003의 기존 RCV-A/B/C 비교를 승인하거나 대체하지 않는다.

## 7. RPC, ReplyToken and timeouts

### 7.1 One-shot ReplyToken 후보

~~~text
Client: Prepare -> Queue Request -> Wait -> Replied
                               \-> DeadlineExpired / LocalCancel / PeerDied
Server:             Receive -> Work -> Reply(token) -> TokenConsumed
                               \-> Dies / TokenRevoked / LateReplyRejected
~~~

Kernel이 승인된 Call을 위해 만든 ReplyToken은 **client Task/Thread, request generation, kernel Endpoint object/generation, grant scope**에 결박한다. Supervisor의 service incarnation/registry 이름을 커널이 해석할 필요는 없다. 새 서비스에 옛 kernel grant를 발행하지 않는 것은 Supervisor의 별도 책임. Server가 임의 숫자를 조작해 타 Task의 request에 응답하거나 여러 번 reply를 수신한 것처럼 꾸밀 수 없어야 한다.

- ReplyToken은 one-shot. reply commit, client death, scoped revoke, 명시적 close 등으로 상태가 전이됨.
- Server가 async worker에 위임하려면 기본적으로 **같은 service incarnation 안의 최소 scope WorkerTransfer**만 연구하고 무조건 임의 Task로 넘기는 것은 금지.
- reply 본문은 별도 SABI bytes이고, kernel은 원래 client에게 정확한 completion을 전달할 권한만 검사한다.
- 숫자 request ID의 재사용으로 늦은 reply가 새 Call의 응답으로 들어가지 못하도록 generation/one-time token을 사용한다.
- Call은 kernel transport의 합성 동작 후보이지 client-visible 외부 비즈니스 transaction이 아니다.

### 7.2 Timeout/Cancellation

- **DEADLINE_EXPIRED**: monotonic-time에 따른 client WAIT 종료. Queue에 이미 커밋된 메시지/remote service effect의 취소를 의미하지 않음.
- **CANCELLED_LOCAL**: client wait 및 token을 로컬에서 정리. Server가 이미 처리한 파일 write 등은 유지될 수 있음.
- **CANCEL_REQUESTED**: SABI가 server에 취소 의사를 전달하는 advisory 메시지. 성공 응답도 명시적인 remote cancel acknowledgement가 아닌 이상 취소 완료로 해석 불가.
- **Late Reply**: Timeout과 Reply commit이 경합하면 하나가 승리하도록 linearization; loser는 명시적 late/cancel error를 받고 구 token으로 새 caller를 깨우지 못함.
- **Exactly-once**: kernel에서 Reply가 한 번만 승인된다는 것과 파일 저장, 네트워크 전송이 정확히 한 번 발생한다는 것은 전혀 다르다.

### 7.3 CPU scheduling & Reply

seL4 MCS는 Call/Reply와 CPU Scheduling Context를 연결한다 [S2]. 그러나 UniOS에서 IPC 대상 서버에 무조건 CPU time/priority를 전달하면 공격자가 높은 권한의 자원을 오용할 수 있다. **RFC-005 합동 검토 전에는 자동 budget donation을 고정하지 않는다.** Opt-in scope, maximum depth, budget owner, timeout/reply/death return, multi-hop cycle 방지와 scheduling fairness를 검증한다.

## 8. Event/WaitSet and notification semantics

### 8.1 Level-triggered readiness proposal

WaitSet은 **level-triggered 상태 조건** + registration generation을 후보로 하되, **W1: one-shot observer+rearm** 또는 **W2: persistent-level observer+coalesced ready-set**을 별도로 비교한다. Zircon zx_object_wait_async는 W1의 예이며 W2와 동등하지 않다. Channel READABLE/WRITABLE, PEER_CLOSED, Event signaled, deadline/timer, service generation lost, cancelled wait 등을 *각 object 타입에서 정의한 조건*으로 관찰한다.

- register + 상태 확인 + arm 사이의 lost-wakeup 경합을 하나의 atomic synchronization discipline으로 해결해야 함.
- readiness를 관찰했어도 다른 receiver가 큐를 먼저 비울 수 있다. Receive가 WOULD_BLOCK을 반환하면 다시 조건을 확인해야 함.
- unregister/close/revoke 후 queue에 남은 이벤트는 registration generation/epoch로 stale delivery 방지.
- 중복 등록/event storm은 per-Task/WaitSet quota 적용. W1 재등록의 missed wake, W2의 계속 true인 조건으로 인한 spinning/repeated packets, close 후 stale completion을 각자 검토한다. Exactly-once event 보장 금지.
- edge-triggered mode는 rearm 누락과 lost-wakeup 리스크가 커서 별도 재검토 후 선택.

### 8.2 Event notification은 counting semaphore가 아니다

Event.signal은 상태 bit 또는 선택된 generation 변경의 힌트다. 다수 signal은 **coalesce될 수 있다**. 정확히 N개의 생산/소비 단위가 필요하면 Channel 메시지 또는 sequence가 검증된 shared-memory queue를 사용한다. seL4 notification도 badge 비트 OR/coalescing을 문서화한다 [S3].

Event WAIT 여러 개가 동시에 기다리는 경우, wake one/wake all은 object 종류와 용도에 따라 명시해야 한다. Wake가 큐 소유권을 자동 이전하는 것은 아니다. Event 자체를 peer 인증 수단으로 취급하지 않는다.

### 8.3 Monotonic clock / wait ordering

Deadline은 monotonic source 기반. 시간 단위·absolute vs relative·suspend resume·clock slack은 RFC-005/009/018의 detail contract. timeout과 readiness 발생이 동일 순간에 경쟁하면 단일 linearization을 정의한다. Woken thread는 readiness 재검사. 상대 객체 닫힘/권한 revoke는 단순 timeout이나 정상 signal과 별도 오류.

## 9. Backpressure and accounting

| Resource | Proposal | Attack/failure class |
|---|---|---|
| direction queue bytes/messages | 유한 queue credits + sender/queue sponsor | flood, memory exhaustion |
| in-transit grants / lineage anchors | explicit kernel metadata budget | privileged cap laundering, queue leak |
| receiver handle slots | receive-time reservation or defined pre-reserve | half-installed rights on OOM |
| outstanding ReplyToken/calls | Task/service instance charge | abandoned RPC overload |
| WaitSet registrations and completion | registration/packet budget | infinite watchers and wake storms |
| Send/receive protected tickets | bounded lifetime + sponsor + cleanup | UNKNOWN state metadata leak |
| shared MemoryObject maps/pins | object sponsor + consumer mapping charges | oversized zero-copy/copy-on-write pressure |
| wired Pager/Boot emergency path | independent reserved pool | total service deadlock |

- **Queue full ≠ Peer closed.** WOULD_BLOCK/QUOTA_EXCEEDED/PEER_CLOSED를 구별한다. Zircon IPC limit는 정책 예외를 사용할 수 있어 UniOS WOULD_BLOCK와 동일하지 않다 [S13].
- WOULD_BLOCK tight retry는 livelock/DoS가 될 수 있다. SDK는 bounded backoff·wait readiness·SABI credits/cancellation을 검토하고, WRITABLE이 모든 크기의 메시지 성공을 보장한다고 주장하지 않는다.
- 커널 자체 대기에서는 취소/권한 회수/peer death 중도 해제 가능해야 하며 unbounded lock hold/recursive queue drain 금지.
- 예약 charge는 commit 전에 확보하며, commit 이후에는 실제 queue/references/cleanup이 끝날 때만 해제.
- FIFO ordering만으로 서로 다른 sender 간 starvation이 사라지지 않는다. Per-producer budget/credit 또는 fair scheduling 후보 평가.
- critical service에 예약된 큐/스택/메시지 공간을 일반 사용자 작업이 소진할 수 없어야 함.
- **max bytes/handles, queue depth, timeout 기본값은 미확정**; 실제 장치/프로파일·TCB 증거 이후 결정한다.

## 10. Close, service death, restart, and revocation

### 10.1 Endpoint close vs peer closed

E1에는 동일 endpoint의 복제 handle이 없지만, endpoint가 **in-transit**이거나 허가된 kernel invocation reference가 존재하는 상태와 Task-local handle 종료는 다르다. PEER_CLOSED는 최종 참조·상태 기계를 고려해 정의한다. 마지막 유효 endpoint 참조와 kernel queue/fault references의 관계를 정의해야 한다.

- **Graceful shutdown 후보:** 새 send를 차단하고 커밋된 inbound queue를 drain한 뒤 EOF/PEER_CLOSED 관측. 단, 대상 큐 방향·timeout/abort escalation 필수. Zircon의 마지막 endpoint close는 해당 endpoint의 unread queue를 파기한다 [S4].
- **Abort/Task kill 후보:** 남아 있는 queue 메시지를 취소·폐기, cap lineage 및 charges 정리; caller remote side effect 결과는 UNKNOWN일 수 있음.
- 구현이 둘을 명시하지 않는 한 graceful flush를 보증할 수 없다.
- 큐에 이동한 handles는 Kernel의 in-transit references이며 상대 앱 Task kill에 의해 자동으로 최초 송신자에게 복귀하지 않는다.

### 10.2 Service incarnation isolation

Supervisor/launcher가 승인한 새로운 service instance는 **새 endpoint/cap grant와 generation**을 받는다. Kernel object generation과 user-space service incarnation은 별개로 구분하고 옛 권한이 새 instance로 자동 연결되지 않게 한다. Old ReplyToken/WaitRegistration/completion은 새 instance와 결합되지 않음. Registry 이름이 같다는 사실은 권한도 세대도 같다는 뜻이 아니다.

Client가 재시도하려면 명시적 re-discovery 및 새 grant가 필요. 이미 commit된 파일/외부 부작용이 불확실하면 서비스 내 transaction ID와 내구적 recovery 정보로 확인해야 한다. Kernel의 완료 통지는 durable storage acknowledgement가 아니다.

### 10.3 Revoke barrier and transit

RFC-002의 B 단계 이후 descendant grant는 큐 안에서도 새 사용을 할 수 없다. 커밋 전에 유효했던 cap이 receiver에 도착했다는 이유만으로 revoke를 우회하지 못한다. R1에는 관련 page mapping cutoff, R2-F에는 작업·DMA fence가 필요. IPC 큐에서 해당 cap을 폐기했다고 entire object의 R2-O 완료를 의미하지 않는다.

## 11. Shared memory and bulk messages

작은 제어 메시지는 Channel, 큰 버퍼는 MemoryObject map + 별도 control/ownership protocol로 연구한다. User-space buffer의 size/offset/producer-consumer index는 **악성 입력**일 수 있어 권한과 memory access bounds를 커널/프로토콜 각각 검증해야 한다.

- 권한은 MemoryObject.map/read/write, Task scope, offset/length, grant epoch로 제한한다.
- Writer가 데이터와 큐 인덱스를 동시에 수정할 수 있으므로 memory-ordering/TOCTOU, overwrite/ABA, stale pin, consumer crash 처리가 필요.
- “zero-copy”는 latency/throughput의 가설일 뿐. 처리 중 버퍼를 복사해야 안전한 설계라면 우선 복사한다.
- 장치 DMA read/write는 CPU 공유메모리와 별개로 IOMMU, pin/unpin, reset, interrupt fence 검증 필요(RFC-007).
- 기존 page mapping을 소유한 Task가 RPC lease 만료 이후에도 볼 수 있다면 service-only revocation으로 strong memory isolation을 주장하지 못한다(RFC-002).

## 12. Sync deadlocks, Pager and lock ordering

~~~text
App.Call(VFS) -> VFS.Call(Pager) -> Pager.Call(VFS) => cycle
App.Call(Desktop) -> Desktop.Call(Input) -> Input.Call(App) => possible cycle
Driver.Call(Pager) -> Pager.Call(BlockDriver) => bootstrap deadlock risk
~~~

- 전체 시스템 RPC 그래프의 모든 순환을 절대 금지할 필요는 없으나 **boot/Pager/recovery의 필수 경로에는 재귀 대기 cycle이 없어야 함**.
- Kernel lock 보유 상태에서 user pointer copy-fault, external service IPC, long queue drain 또는 unbounded revocation traversal 금지.
- Pager의 code/stack/IPC message/critical WaitSet/Reply paths가 자기 자신에게 page-in을 요청하지 않도록 wired/예약 메모리 경계.
- 동기 RPC의 nested chain은 deadline propagation, cancellation, priority inversion, lost reply, service starvation을 모델링해야 함.
- 긴 I/O는 async job + completion SABI로 전환하는 후보를 유지. 무조건 동기 call을 사용하지 않음.
- Rust sync primitives/user-space libc mutex는 커널 IPC 권한/수명 보증을 대체하지 않음.

## 13. Error taxonomy (논리적, 값 미확정)

| Result | 뜻 | 잘못된 추론 금지 |
|---|---|---|
| NOT_COMMITTED | 이 IPC enqueue와 MOVE가 발생하지 않음 | 모든 이전 외부 작업이 없었음 |
| COMMITTED | queue의 메시지/handles 이동만 확정 | receiver delivery, processing, fsync |
| UNKNOWN | 호출자가 진짜 commit 여부를 모름 | safe blind retry |
| WOULD_BLOCK | 현재 큐/대기 조건이 준비되지 않음 | peer crash |
| QUOTA_EXCEEDED | 지정 sponsor/resource limit 초과 | 전역 물리 메모리 모두 소진 |
| PEER_CLOSED | 같은 instance의 상대 endpoint 닫힘 | 서비스 부작용이 없었음 |
| DEADLINE_EXPIRED | local wait가 해당 시한 안에 응답을 못 받음 | 원격 작업 중단 |
| CANCELLED_LOCAL | local wait/reply permission 정리 | 원격 write 성공적으로 취소 |
| REVOKED | 해당 grant lineage 신규 접근 불가 | object-wide reclaim 완료 |
| STALE_INSTANCE | 이전 서비스 인스턴스의 결과/권한 | 새 instance에서 자동 유효 |
| BUFFER_INVALID | 크기/pointer/출력 준비 오류 | 이미 확정된 커밋 rollback |
| SCHEMA_UNSUPPORTED | user SABI 프로토콜 협상 불가 | kernel ABI/type 자체가 손상 |

Errno/NTSTATUS/Mach 오류는 사용자 공간 personality가 번역한다. Native SDK 오류 코드/상수/시그니처는 RFC-009/018.

## 14. Invariants: 32 proof obligations, 모두 Proposed

| ID | 검증해야 할 불변식 |
|---|---|
| IPC-01 | SEND/RECEIVE/REPLY/WAIT 각각의 typed rights·Task scope를 커널이 확인. |
| IPC-02 | 커밋된 메시지의 bytes + moved grants는 불가분 queue record. |
| IPC-03 | NOT_COMMITTED인 전송은 모든 sender 원본 grants와 예약 charges를 보존. |
| IPC-04 | COMMITTED 뒤 이동된 원본 sender handles는 다시 유효해지지 않음. |
| IPC-05 | Result 유실 뒤 COMMITTED를 거짓 NOT_COMMITTED로 보고하거나 blind retry 허용하지 않음. |
| IPC-06 | Receive 단계에서 모든 grants가 설치되거나 아무것도 설치되지 않음. |
| IPC-07 | In-transit handles의 lineage, rights, scope, sponsor obligations가 보존됨. |
| IPC-08 | Revoke B barrier 이후 queued descendant가 새 권한으로 되살아나지 않음. |
| IPC-09 | User receive copyout 오류가 partial cap-table install을 초래하지 않으며, 민감 내용이 이미 공개될 수 있었던 메시지를 별도 수신 Task/ServiceIncarnation에 무단 자동 재전달하지 않음. 공개 전후 바이트 관찰은 구분하며 **미완료 수신이 비공개였다고 주장하지 않음** (Proposed). |
| IPC-10 | Kernel queue commit을 remote service의 완료/내구성과 혼동하지 않음. |
| IPC-11 | ReplyToken은 kernel-minted, scoped, generation-bound, one-shot. |
| IPC-12 | Late/stale reply는 새로운 call이나 다른 Task를 깨우지 않음. |
| IPC-13 | Timeout/local cancel이 remote side effect 취소로 잘못 보고되지 않음. |
| IPC-14 | 서비스 재시작 시 old callback/cap/reply가 새 incarnation으로 전환되지 않음. |
| IPC-15 | Readiness는 재검사 가능하고 경쟁 후 WOULD_BLOCK을 정상 처리. |
| IPC-16 | register/arm과 signal 사이 영구 lost wakeup 없음(선택된 level-triggered semantics). |
| IPC-17 | unregister/close/revoke 이후 stale registration 이벤트가 다른 대상에 전달되지 않음. |
| IPC-18 | queues, grants, reply, receipt, WaitSet, cleanup은 sponsor quota로 bounded. |
| IPC-19 | 악성 사용자 Task는 boot/Pager recovery reserved IPC pool을 소진하지 못함. |
| IPC-20 | Endpoint SEND/RECEIVE가 타 Task 임의 메모리/DEBUG 권한으로 확대되지 않음. |
| IPC-21 | Kernel-minted provenance와 user-supplied identity 문자열이 혼동되지 않음. |
| IPC-22 | Shared MemoryObject 및 DMA는 별도 map/device grants와 revoke/fence를 필요로 함. |
| IPC-23 | SMP의 send/receive/close/revoke는 설명 가능한 linearization을 가짐. |
| IPC-24 | cleanup/wait/revoke 경로의 kernel lock hold와 queue drain은 bounded. |
| IPC-25 | Linux/Windows/Darwin guest 권한 숫자가 native IPC grant를 자동 부여하지 않음. |
| IPC-26 | Unknown/malformed SABI method는 kernel/object ownership 또는 memory isolation을 우회하지 못함. |
| IPC-27 | E1 Endpoint 단일 소유는 Task 간 MOVE와 in-flight syscall의 선형화로 유지된다. |
| IPC-28 | 민감 payload/metadata 공개 전에 적법한 Claim·수신 audience를 검증하고, 공개 가능성 표시 이후에는 같은 message를 새 권한 주체에게 무단 재큐잉하지 않는다. 이미 관찰한 B의 정보/외부 전송은 복원할 수 없으며 이 조건은 전체 데이터 비공개 증명이 아님 (Proposed). |
| IPC-29 | Kernel ReplyToken generation과 Supervisor service incarnation 혼동이 stale 권한을 새 서비스로 확대하지 않는다. |
| IPC-30 | WaitSet W1/W2 각각 재등록/지속 알림, stale packet, 중복 observer와 busy-loop 규칙이 명확하다. |
| IPC-31 | WOULD_BLOCK busy retry와 queue/credit 제한을 악용한 무제한 privileged work 또는 critical reserve 고갈이 방지된다. |
| IPC-32 | 동일 send에서 송신 Endpoint 자체 MOVE가 차단되고 순환 Endpoint graph에 bounded cleanup 또는 제한 정책이 존재한다. |

각 속성은 전제(지원 CPU/펌웨어/잠금/메모리), 실패 모드 및 수학적/테스트 검증 방법을 명시해야 한다. 아직 formal verification이나 running test를 통과한 속성은 하나도 없다.

## 15. Failure / race matrix

| 시나리오 | 금지 결과 | 필요한 확인 |
|---|---|---|
| Invalid sender buffer / syscall fault | half-moved handles | commit 없음, sender 보존 |
| Commit 후 syscall 결과 분실 | 무조건 retry, cap 중복 | UNKNOWN + reconcile/fail-closed |
| 동일 handle 반복 MOVE | duplicated authority | atomic reject |
| Send와 revoke B 동시 실행 | revoked cap 재전송 우회 | 이전 commit 또는 이후 reject; receiver recheck |
| Receive user copyout fault | queue/cap ownership 유실 | escrow/rollback/정의된 error |
| Receiver handles quota 부족 | partial grant install | all-or-none |
| Timeout vs reply commit | stale reply가 새로운 caller 완성 | one-shot generation, definite winner |
| Server death 후 즉시 재시작 | old ReplyToken 새 서비스로 적용 | incarnation scoping |
| WaitSet register vs signal | 영구 lost wakeup | atomic arm + state observation |
| WaitSet unregistration vs event delivery | stale target/ABA | registration generations |
| Channel endpoint cycle | kernel refcount leak, 재귀 cleanup 폭주 | quota + iterative collector 또는 feature gate |
| Queue full producer storm | 무제한 CPU/메모리 | bounded queue, credits/fairness |
| Pager own IPC page fault | 교착/회복 불능 | wired reserve and dependency DAG |
| Cancel after remote file write | 앱이 실제 I/O가 없다고 오인 | remote effect unknown + reconciliation |
| Over-aggressive priority donation | 다른 Task CPU time 훔침 | bounded opt-in donation or disabled |
| Share buffer index racing | 데이터 침범 | offset/length/ordering validation |
| DMA revoke during IPC | reuse unsafe frame | R2-F fence or quarantine |

## 16. Alternative study and gates

| Candidate | 장점 | 리스크 | 선택 증거 |
|---|---|---|---|
| **IPC-A async Channel/Reply/WaitSet** | desktop/GUI/service 작업 적합 | queue/receipt/reply/wait TCB 증가 | ownership, charge, deadlock proof and later measured costs |
| **IPC-B seL4-like rendezvous-first** | 작은 kernel queue, sync fast path | GUI event/pager/worker가 복잡, synchronous wait graph | same workload+security benchmark |
| **IPC-C completion queue/async RPC** | many outstanding requests 처리 용이 | user method semantics가 kernel로 새어 들어갈 위험 | strict KABI independence and lifecycle model |
| **IPC-D shared-memory-only** | potential bulk data speed | 권한/notification/backpressure가 별도 필요 | access/auth/transaction contract not by raw bytes alone |

**현재 판단:** IPC-A가 가장 실용적인 연구 기본안. 단, send/receive copyout atomicity 및 ReplyToken/receipt bounded lifecycle을 증명할 수 없다면 예산/기능을 축소하거나 IPC-B를 재평가한다. 첫 연구 단계에서 fast path 최적화로 안전성을 희생하지 않는다.

## 17. Verification and future measurement protocol

**먼저 설계 모델 검사:** 2~4 Tasks, 각 방향 queue, 두 개 grant roots, 1~3 handle MOVE, single reply token, 1 WaitSet registration, budget exhaustion, usercopy fault, deadline/reply race, server restart, revoke B를 유한 상태로 탐색. TLA+/Alloy/PlusCal 또는 동등 형식으로 검토. 선택한 도구가 있다는 것만으로 검증 성공을 주장하지 않는다.

- Safety: IPC-01~32, ownership conservation, no privilege gain, single reply authorization, no stale wake.
- Liveness: fair scheduling/finite bounded resources 전제 하 cleanup starvation, pager-self-deadlock, abandoned waiter 방지.
- Linearization: enqueue-vs-B revoke; dequeue-vs-usercopy error; timeout-vs-reply; wait-register-vs-signal; close-vs-receive.
- Accounting: sponsor death, queue drain, retry/ticket expiration, cyclic endpoint references.
- Negative test plans: malformed descriptors, handle duplicate, cross-Task ID guessing, parser fuzz, unknown SABI ordinal, unbounded nesting, revoked endpoint.

**별도 구현 Go 이후에만** 실행/측정 가능한 목표:
- 0/64/256/4096 bytes, 0/1/다수 cap, same vs cross core, p50/p95/p99, syscall/IPC/GUI whole-path latency.
- Queue zero/full, worker death/restart, receipt unknown, reply timeout, WaitSet storms, shared-memory copy vs map.
- 1/100/10000 registration/cap fan-out, latency tails, kernel memory per message/ticket/reply, fairness under flood.
- QEMU/KVM/TCG와 실물 Reference PC를 분리하고 CPU mitigation/interrupt/noise/clock을 기록.
- RFC-001의 64B IPC p50≤5µs/p99≤20µs 같은 값은 **미검증 설계 가설**이지 지원 SLA가 아님.

## 18. Cross-RFC dependencies

| Related RFC | 필요한 공통 계약 |
|---|---|
| RFC-002 | Typed caps, transaction MOVE, sponsor, revocation B/R1/R2, ReplyToken 권한 및 lineage |
| RFC-004 | Usercopy page-fault safety, shared MemoryObject mapping, Pager wired pool, teardown |
| RFC-005 | Deadline, thread scheduling, priority inversion, optional budget donation/return |
| RFC-006 | Initial capabilities, recovery service DAG, service generation, peer-close cleanup |
| RFC-007 | Device-driver IPC, IOMMU-group buffer pins, IRQ, DMA/timeout fence |
| RFC-008 | fsync/durability and idempotent external work after uncertain completion |
| RFC-009 | KABI signature, syscall return/result-copyout semantics, native SDK |
| RFC-010/011/012 | Guest Linux/Windows/Darwin API translation outside kernel |
| RFC-015 | Threat model, fuzzing, benchmark gates, trace security |
| RFC-016 | Service/portal consent/auth identity and lease policy |
| RFC-018 | KABI protocol versioning, SABI IDL/unknown-method policy |

**P0 공동 리뷰 묶음:** (RFC-002+003+009) Commit/result ambiguity; (003+004+006) Pager fault/deadlock; (003+005) Reply/CPU donation; (002+003+007) in-transit cap vs DMA revoke.

## 19. Open issues (26)

| ID | Priority | Unresolved decision | Cross-RFC |
|---|---|---|---|
| I3-01 | P0 | How to report a committed send if syscall return is lost? | 002/003/009 |
| I3-02 | P0 | ReceiveEscrow vs copy-before-commit vs register-only return? | 003/004/009 |
| I3-03 | P0 | Queue-grant transfer racing ancestor revoke B? | 002/003 |
| I3-04 | P0 | Queue/Reply/Wait/ticket sponsor debt after Task death? | 002/003/004 |
| I3-05 | P0 | Critical Pager/Boot wired IPC DAG and lock order? | 003/004/006 |
| I3-06 | P0 | Timeout/client death vs late one-shot ReplyToken? | 002/003/006 |
| I3-07 | P0 | Multi-owner endpoint close; graceful drain vs abort semantics? | 002/003/006 |
| I3-08 | P0 | Bounded GC for cyclic endpoint references? | 002/003 |
| I3-09 | P0 | WaitSet signal registration/unregistration lost wake? | 003/005 |
| I3-10 | P1 | Kernel ReplyToken or user-space reply channels? | 003/005 |
| I3-11 | P1 | Per-producer credit and starvation vs FIFO? | 003/015 |
| I3-12 | P1 | Trusted badge provenance privacy/type/forgery? | 003/015/016 |
| I3-13 | P1 | Ticket lifespan, revocation, authorized status inspection? | 002/003 |
| I3-14 | P1 | Message/cap/queue size limits and negotiation? | 003/009/018 |
| I3-15 | P1 | Shared-buffer memory ordering/coherence and cleanup? | 003/004 |
| I3-16 | P1 | Monotonic absolute/relative deadline, suspend/timer slack? | 003/005/009 |
| I3-17 | P1 | Scheduling-context donation bound and return on timeout? | 003/005 |
| I3-18 | P1 | Strict/flexible SABI schema upgrade and idempotent reconnect? | 016/018 |
| I3-19 | P1 | Native errors vs guest errno/NTSTATUS, no kernel UAPI pollution? | 009/010/011 |
| I3-20 | P2 | DMA/IO rings, fastpath and co-location after evidence? | 003/007/013 |
| I3-21 | P0 | E1 exclusive Endpoint vs E2 multi-owner, handoff and service fan-in? | 002/003/006 |
| I3-22 | P0 | Receive pre-commit disclosure/Claim and copyout racing other readers? | 002/003/004/009 |
| I3-23 | P0 | Kernel ReplyToken vs Supervisor service incarnation, stale reply rights? | 002/003/006 |
| I3-24 | P0 | WaitSet W1 one-shot vs W2 persistent, rearm/lost wake/stale packets? | 003/005 |
| I3-25 | P0 | Async queue flow control, sender credits, WOULD_BLOCK livelock/fairness? | 002/003/015 |
| I3-26 | P0 | Self endpoint MOVE and cyclic ownership, bounded nonrecursive cleanup? | 002/003/006 |

## 20. Decision proposals (16, not yet Accepted)

- **IPC-D01 [revised]:** IPC-A bounded Channel with E1 exclusive Endpoint, one-shot ReplyToken and W1/W2 WaitSet research is baseline; E2/IPC-B/C remain alternatives.
- **IPC-D02:** Kernel Envelope and versioned user-space SABI/IDL must remain distinct; no guest syscall semantics inside native kernel IPC.
- **IPC-D03:** Small control messages transfer kernel-verified scoped cap metadata; exact wire layout/count is not frozen.
- **IPC-D04:** Send queue append and sender handle removal have one commit; receive install/dequeue is a separate commit.
- **IPC-D05:** Precommit failure retains sender handles; committed/lost result requires protected status or fail-closed handling.
- **IPC-D06:** Kernel-origin scoped one-shot ReplyToken is an initial hypothesis to block late-response confusion.
- **IPC-D07:** Timeout/local cancellation cannot be represented as rollback of remote effects.
- **IPC-D08 [revised]:** Prefer level-ready conditions with registration generation, but select W1 one-shot/rearm vs W2 persistent before ABI freeze; Signal is not a precise count.
- **IPC-D09:** Every queue, Reply, ticket, WaitSet, transit cap has bounded sponsor budget, with protected critical reserves.
- **IPC-D10:** MemoryObject is optional bulk-data path, not universal zero-copy/safety proof.
- **IPC-D11:** Service incarnations and old scopes are distinct, reconnection requires explicit new grants.
- **IPC-D12:** Budget donation and fast-path optimizations are deferred to RFC-005 and evidence-driven later decisions.
- **IPC-D13 [new]:** E1 ChannelEndpoint는 복제/파생 불가, 자기 전송 MOVE 금지; E2 멀티소유는 별도 연구.
- **IPC-D14 [new]:** RCV-B receive-before-commit usercopy는 독점 Claim/권한 보장이 없으면 채택 금지.
- **IPC-D15 [new]:** ReplyToken kernel scope와 Supervisor service incarnation은 분리.
- **IPC-D16 [new]:** W1/W2, flow control과 cyclic endpoint cleanup의 liveness를 IPC-A 승인 전 검증.

**Status discipline:** These are Proposed Specification choices only. D1–D8 and ADR-002 stay Accepted — Direction; RFC-001/002 baseline is not silently amended. This RFC does not mean kernel code, ABI, hardware support, verified security or implementation Go.

## 21. Official primary sources (checked 2026-10-24)

- **[S1] seL4 IPC tutorial:** https://docs.sel4.systems/Tutorials/ipc
- **[S2] seL4 MCS and scheduling-context donation:** https://docs.sel4.systems/Tutorials/mcs.html
- **[S3] seL4 Notifications and shared memory:** https://docs.sel4.systems/Tutorials/notifications.html
- **[S4] Zircon Channel kernel object:** https://fuchsia.dev/fuchsia-src/reference/kernel_objects/channel
- **[S5] Zircon zx_channel_write:** https://fuchsia.dev/reference/syscalls/channel_write
- **[S6] Zircon zx_channel_call:** https://fuchsia.dev/reference/syscalls/channel_call
- **[S7] Zircon Signals and readiness:** https://fuchsia.dev/fuchsia-src/concepts/kernel/signals
- **[S8] Zircon zx_object_wait_async:** https://fuchsia.dev/reference/syscalls/object_wait_async
- **[S9] FIDL language/version/unknown interactions:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/language
- **[S10] FIDL wire format:** https://fuchsia.dev/fuchsia-src/reference/fidl/language/wire-format
- **[S11] Zircon zx_channel_write_etc:** https://fuchsia.dev/reference/syscalls/channel_write_etc
- **[S12] Zircon zx_object_wait_one:** https://fuchsia.dev/reference/syscalls/object_wait_one
- **[S13] Fuchsia Zircon IPC Limits:** https://fuchsia.dev/fuchsia-src/concepts/kernel/ipc_limits
- **[S14] Zircon zx_channel_read:** https://fuchsia.dev/reference/syscalls/channel_read

**Research quality note:** External source documentation is evidence about external platforms only. UniOS-specific protocols, error states, budgets and security invariants are novel design proposals and have no runtime/formal/benchmark validation. No third-party kernel source was imported.

## 22. Critical re-review record: v0.1 → v0.2

**Evidence and limits:** Official seL4 IPC/MCS, Zircon Channel/Read/Call/Wait and IPC Limits were checked alongside RFC-001/002. This is a document-level adversarial counterexample review, NOT formal verification, independent audit, implementation or performance validation.

| Finding | v0.1 vulnerability | v0.2 research change |
|---|---|---|
| Channel ownership | Assumed duplicated/shared Endpoint is ordinary | E1 exclusive-ownership candidate, E2 optional |
| Receive data secrecy | Bytes can leak to loser of receive race before commit | Claim/auth before user disclosure; RCV-B default rejected |
| Service identity | Kernel ReplyToken could be tied to user-space registry identity | Scope Kernel object/generation separately from Supervisor incarnation |
| Wait observation | Zircon one-shot wait mistaken for persistent Level WaitSet | W1 one-shot rearm vs W2 persistent level |
| Async memory pressure | Queue bounds / WOULD_BLOCK alone imply stability | IPC limits, flow control, no tight retry, critical reservation |
| Endpoint self-transfer | Sender endpoint could be moved in own send; cycles unclear | Reject self MOVE, cycle proof or feature gate |
| Graceful shutdown | Drain direction/deadline not stated | Explicit grace/abort escalation and in-transit state |
| Source behavior | Zircon policy exception differs from UniOS errors | Mark external documentation vs independent UniOS proposal |

**Proof blockers:** E1 MOVE vs concurrent invocation; claim/disclose/dequeue with untrusted usercopy; W1/W2 wait register race; bounded receipt/ticket and Reply lifecycle; end-to-end flow-control; Task kill with cyclic endpoints. These must be tested using model checking and later separately approved fault injection. Never treat a design invariant as a proven feature.

**Decision:** Retain IPC-A as conditional baseline. RFC-002 type-specific capability rights allow ChannelEndpoint nonduplication as a candidate without changing D1–D8. Kernel/SDK code unchanged; RFC-003 remains Proposed Specification.

---

**Document history:** 2026-10-24 v0.1 initial study (Git history retained). 2026-10-24 v0.2 corrections of ownership, confidentiality, WaitSet/reply scopes, pressure and lifecycle; documentation only.


## 23. 2026-10-25 joint receive state-model re-review addendum — Proposed only

**Scope / authority:** This addendum compares §§5–6,10,13–15,19–20 with the [CAPIPC unified state model v0.2](../models/capipc-send-receive-revoke-state-model-v0.2.md). The earlier [v0.1](../models/capipc-send-receive-revoke-state-model-v0.1.md) and [CAPIPC integration contract v0.2](../contracts/capability-ipc-sabi-integration-contract-v0.2.md) remain historical/parallel Proposed artifacts. It is an independent design-paper cross-check, NOT acceptance of E1/RCV-A/RCV-T1/RCV-T2, a new syscall/ABI, executable formal proof, OS code, test or security certification.

### 23.1 Exact ownership and disclosure distinctions

| Boundary | RFC-003 existing emphasis | Additional candidate safety cut (P0 OPEN) |
|---|---|---|
| SendHalf and ReceiveHalf | distinct E1 endpoints, one public Task owner per half | claim generation and request admission must also track which half MOVEd |
| K-S | one queue append + complete source-cap MOVE; NOT_COMMITTED vs UNKNOWN | preidentified protected receipt even if new ticket number/report lost |
| K-C | exclusive receiver ReceiveEscrow candidate | issuer-authorized fixed recipient audience bound to Task+Endpoint+ServiceIncarnation and current policy |
| K-Vm | not separately named | metadata lengths, cap counts, type/rights and size errors can be policy-sensitive even when no payload bytes are copied |
| K-Vp | partial copyout recognized as dangerous | mark possible exposure **before** any user-visible payload store/map; no cross-audience auto-redelivery thereafter |
| K-I | cap-vector all-or-none install plus queue dequeue | user-facing handle-number publication **not** automatically atomic with kernel cap install |
| Cap usability | actual installed cap is a native authority | whether public token visible/usable before successful SABI method decode must match declared service security profile |
| K-U optional T2 | not part of original RCV-A proposal | conditional inert cap escrow **only with independently trusted verifier/issuer** and live revoke+policy recheck at activation |
| Terminal settlement | abort/timeout/death and bounded charge | disclosed message may be lost rather than requeued; orphan cap/table slot, token tombstone and DMA pin charge separately |
| Revocation B | queued descendant cannot become newly usable | in-claim/install/activation cut must revalidate lineage and distinguish admitted old effects from new rights |

### 23.2 Counterexamples the original §6 wording could hide

1. **Handle-number publication race:** kernel writes first token to B user memory, faults writing second, and no cap installation occurs. The first raw token may be held by B; it must never later authorize an unrelated grant. Conversely if K-I happened first and token copyout faults, active orphan caps and charge/authorized cleanup remain.
2. **Confidential metadata:** a receiver probes message size/cap count before valid K-C. A generic BUFFER_TOO_SMALL result can reveal secret message structure; current RECEIVE rights/audience and issuer's metadata policy are independently required.
3. **Endpoint ownership race:** B owns ReceiveHalf and acquires K-C; ReceiveHalf MOVEs to C before K-Vp. Do not silently write to old B or treat C as the same authorized recipient. Unexposed claim can be revalidated/aborted; exposed claim cannot silently redeliver.
4. **Stale T2 release:** a trusted verifier checked grant under policy P0, controller publishes B and server policy becomes P1, then caller invokes K-U. Old positive verdict alone cannot activate revoked authority.
5. **Spurious all-or-none conclusion:** an atomic cap-vector installation says nothing about B's earlier 16-byte payload observation, future external forwarding, service effect or complete handle-array copyout.
6. **Cleanup completion confusion:** after exposed claim terminalizes, an unknown DeviceCap might be closed while DMA continues. This is not R2-F, R2-O, persistence or retroactive confidentiality.

### 23.3 Proposed constraints, not approved wire rules

- **IPC-09/28 corrected scope:** no misleading guarantee that unsuccessful receive revealed no data; forbid uncontrolled automatic redelivery to *another distinct recipient* after possible disclosure. An authorized B's earlier memory observation cannot be erased.
- **Metadata policy:** define public or confidential projections for size/count/type/rights; if confidential, authorization and monotonic visibility accounting apply to metadata too. A hidden size query cannot bypass RECEIVE scope.
- **Token lifecycle:** separate slot reservation, public token copying, kernel install, first invokability, error reporting and protection against stale bits. Do not select token-first or install-first until both failure paths are modeled.
- **T1 versus T2:** RCV-T1 remains lower-TCB future proof candidate if grant was already legitimately usable by B. T2 is a composable optional promise, valid only if a separate issuer-authorized verifier can prevent B from releasing its own escrow; K-U must still validate current kernel cap and policy.
- **Sponsor and M0:** pending claim, partial token output, installed-but-unreported cap, terminal receipt, orphan charge and unfenced DMA each require bounded sponsor ownership, independent from ServiceID and Task survival.
- **Result semantics:** SEND_COMMITTED, RECEIVE_CAP_INSTALLED, bytes/metadata POSSIBLY_VISIBLE, right USABLE, METHOD_ADMITTED, external EFFECT_UNKNOWN and storage SYNC_SCOPED are not interchangeable success states.

**Unresolved P0 exit:** exact guarded receive ordering, token publication/activation, claim/audience and policy epoch during Endpoint MOVE, revocation vs K-U, metadata leak bounds, terminal debt cleanup and Pager/M0 liveness. The Proposed E1 right-table DERIVE textual inconsistency fixed previously remains fixed; none of the new P0 items is proved by that editorial correction.

**Roadmap:** review these scoped semantics jointly with RFC-002/009/018 and the CAPIPC model, choose one mathematically precise receive ordering only after a counterexample-free proposed state specification, and seek **separate** authorization if any future executable model or code is desired. Until then RFC-003 remains **Proposed v0.2**.

## 24. 2026-10-25 Handle Publication and Activation joint design study (Proposed, not approved)

**Handoff:** [Handle Publication, Receive Commit & Capability Activation v0.1](../models/capipc-handle-publication-activation-v0.1.md). This is an additional **non-normative Proposed** deep-research cross-reference to RFC-003 §§5–6,9–10,13–14,23. No original IPC-Dxx decision is elevated to Accepted, no kernel executable, syscall number, type width, stable SABI, test or formal model is authorized.

### 24.1 Critical semantics and why K-I needs a narrower promise

One whole-vector K-I receiving commit may guarantee **kernel cap-table installation + queue/claim retirement** atomically. It does **not** guarantee one atomic userspace publication of the corresponding numeric handle array, full data-buffer copyout, result observation by the initiating Thread, or future method/durability success. Accordingly:

- **K-HR:** a scoped, generation-bound pre-reservation of Task-public Handle tokens; inactive IDs may be safely copied before grant installation only if they can NEVER be reassigned to unrelated grants after a partially visible failure.
- **K-HP:** numeric user handle publication may be partial and Task-shared. No native cap rights arise from token bytes alone.
- **K-I:** ordinary bearer-cap receive completion still requires *one* all-or-none native cap vector installation and message/claim retirement, **not a second hidden completion**. Competing revoke B, Endpoint MOVE, Task death, terminalization, and two simultaneous commit attempts must have unambiguous linearization.
- **K-Q:** a trusted preidentified claim/receipt query may establish actual kernel K-I status despite lost return, but cannot prove a userspace Thread read the numbers or a remote FileService completed work. A newly minted claim ID supplied only in the lost return is NOT safely recoverable.
- **K-U optional T2:** a genuinely independent verifier/issuer rechecks current exact kernel grant set, scope, Task/Endpoint generation, PolicyEpoch and revocation at a separate cap activation cut. A server's own unchecked “IDL validated” claim is not security proof.

**HPA-C candidate**, with kernel-private ReceiveClaim escrow, token reservation, staged output and a distinct explicit receiver AcceptCommit, is a *formal proof leader*, not already selected universal kernel API. **HPA-A (install first)** and **HPA-B (single-call token-first)** are legitimate competing research designs, potentially lower overhead; no performance or TCB measurements exist. In T1, explicit AcceptCommit expresses an authorized receiver **Task's bearer-cap acceptance**, not proof of method schema check, user knowledge or human consent.

### 24.2 Failure/race cases to preserve as P0

| Scenario | Safety requirement (not yet proved) |
|---|---|
| First of two numeric public Handle tokens copied, second faults | no active partial vector; exposed token ID not reused into new unrelated authority |
| Kernel K-I succeeds but token/status return is lost | charged live cap rights remain exactly in B Task; unknown to initiating Thread until authenticated query |
| Two Threads race to AcceptCommit same claim | one winning K-I; no doubled vector or queue removal |
| K-V metadata/payload exposed then claimant abandoned | no automatic redelivery to distinct C/B2 audience; terminal cleanup charged |
| Revoke B or Endpoint MOVE during staged claim | K-I revalidates actual rights/Task/Endpoint generation; no post-B new authority |
| Trusted T2 verifier result stale after issuer policy update | K-U requires fresh actual grant and policy validation; fail closed |
| Claim/result ID lost or guessed across Task reincarnation | preidentified or safely enumerable claim status, privacy/anti-ABA boundaries |
| Repeated staged faults use every Task token value or queue credit | bounded admission and fail-closed exhaustion, not insecure reuse or M0 starvation |
| Terminalized DeviceCap references unfenced DMA | R2-F and R2-O remain distinct; sponsor/quarantine cannot be refunded early |

**RFC-003 invariants IPC-06, IPC-09 and IPC-28 remain Proposed** and must be read with §23's clarified possibility of partial user-data exposure. IPC-06 is about native cap-vector installation, not all-or-none usercopy; IPC-09/28 prohibit unsafe new-recipient replay after possible exposure rather than claiming failure erased earlier observations.

**Exit OPEN:** exact public token reservation and permanent nonreuse, initial claim-ID recovery, bounded queue/claim/CPU budget, Task-wide concurrent observation, chosen commit order, honest guest/remote effect mapping and independent T2 verification must be addressed by a future, separately reviewed contract before any ABI approval.

## 25. 2026-10-25 HPA independent adversarial re-review — revised research priority (Proposed only)

**Source:** [HPA v0.2 Independent Adversarial Re-Review](../models/capipc-handle-publication-activation-v0.2.md), updating the research recommendation in historical [HPA v0.1](../models/capipc-handle-publication-activation-v0.1.md). This is a *non-normative RFC-003 alignment addendum*, not an Accepted IPC-Dxx decision, chosen KABI syscall, implementation or formal correctness result. The v0.1 rationale remains intact in Git for comparison.

### 25.1 Corrected ranking and threat boundary

Earlier §24 described **HPA-C two-phase Stage → Task AcceptCommit** as the formal proof leader. Independent review found an ordinary Task's own AcceptCommit does **not** verify SABI schema, original user consent, independent security policy or actual message-byte retention. It merely expresses Task-level bearer-cap acceptance. In a malicious B Task, the extra syscall can be invoked regardless of message correctness. Both C and the simpler candidate still need kernel K-I all-or-none cap installation, token nonreuse, a protected result/claim record, current R-B/revoke checks, K-Vm/K-Vp exposure accounting and bounded cleanup.

**Revised Proposed research ordering:** 
- **HPA-B+ first for ordinary T1 bearer IPC** — a single logical receive, with bounded private staging, noninvokable token reservation, protected result record and a single K-I full-vector install + queue retirement cut after appropriately guarded output publication. Plus denotes stronger proof obligations than unqualified original HPA-B, not a chosen ABI.
- **HPA-C conditionally** for explicitly required deferred activation, application acceptance or **independently trusted** method-policy validation; the last property actually requires separately scoped T2 inert escrow and K-U under live issuer policy, not self-reporting by B.
- **HPA-A remains a comparison candidate** with distinct installed-but-unreported cap cleanup, not a default security guarantee.

This change **revises design advice**, not an E1/RCV method acceptance. §24 accurately records the earlier idea; this §25 is the current *research recommendation*. No sources contradict the fundamental E1 one-owner, native cap-vector atomicity or precommit NOT_COMMITTED candidate; none are proven.

### 25.2 Newly emphasized P0 counterexamples

| Case | Requirement still OPEN |
|---|---|
| Stage returns a new ClaimID only through an invalid output pointer | Cannot claim two-phase result is recoverable; preidentified Task-scoped result slot or safe enumeration required |
| Same Task B-Thread1/2 both call AcceptCommit | Exactly one K-I winner; Task-wide cap use does not imply initiating Thread/human consent |
| B stages confidential request, performs external work with existing rights, then abandons claim | No statement that uncommitted Receive implies backend effect absent |
| B repeats Stage then faults after public token output | Token namespace and queue budget exhaustion must fail closed without M0 starvation |
| B modifies Stage user bytes between verifier decision and K-U | Any trusted T2 release must bind original authenticated immutable message and actual kernel cap metadata |
| A protected status record exists for B+ as well as C | Do not count postcommit status recovery as a unique C security advantage |
| Current policy/revoke B arrives before K-I or K-U | No new invokable revoked rights, regardless of prior staging/verification |
| Result lost after K-I while user tokens already copied | Underlying B native grants and sponsor remain; query or honest UNKNOWN, never pretend K-I rolled back |

**Token reuse nuance:** public values **possibly** copied to user memory are never reused to identify unrelated CapGrants in the same Task. A kernel-reserved value **provably never published and never previously live** might be reclaimed if an independent ABA/namespace proof allows; do not claim every tentative internal reservation must be burned forever. Security authority remains the actual Task-local kernel cap table, not guessing a number.

### 25.3 Exit and repository policy

The detailed KABI call/handle bit format, receipt query namespace, queue budget, issuer provenance and actual memory-order/lock proof all remain P0 OPEN. A two-phase scheme may be worth its complexity only if it establishes a distinct required security or sequencing property under bounded CPU/IPC/M0 costs; no performance measurement establishes a universal winner. SOURCE RFC-002/003/009/018 Proposed specs require joint review before any acceptance.

**No implementation:** this addendum is written contract research only. No kernel/SDK/compiler/protocol/header work or executable formal model was authorized by this change.

## 26. 2026-10-25 Kernel IPC status receipts and Task Handle identity — Proposed cross-reference

**Source:** [CAPIPC Task Handle Namespace & Kernel Receive Receipts v0.1](../models/capipc-task-handle-namespace-and-receipt-v0.1.md), alongside HPA v0.2 and this RFC §§5–7,9–10,23–25. This is **a Proposed research addendum**, not an accepted receipt syscall, numerical ABI, tested formal model or implemented IPC.

**The critical status rule:** actual kernel K-S SEND and K-I RECEIVE commits each need **one protected, pre-reserved and coherent result record** if status reconciliation is promised. A later query can report actual phase, but result-copyout failure does not change ownership. A ticket minted only in the lost return is not a recoverable ticket. Candidate KR-A uses an already known Task-generation-scoped receipt slot or caller-supplied request nonce **as correlation**, with *separate* protected query rights; no SABI RequestId or ServiceID string becomes native authority.

| State | Truthful meaning |
|---|---|
| PENDING | irrevocable cut may still occur; **NOT** final NOT_COMMITTED and NOT a replay permission |
| FINAL_NOT_COMMITTED | that exact kernel operation has terminated before the cut and cannot later commit |
| COMMITTED_KS | whole SEND bytes/cap-vector moved to kernel queue; source tokens invalidated |
| COMMITTED_KI | whole RECEIVE vector installed+message retired; public token array/caller knowledge may still be uncertain |
| TERMINAL | message/claim settled without K-I; prior payload exposure or remote effect not necessarily absent |
| EXPIRED_EVIDENCE | original protected proof unavailable after bounded GC; cannot infer clean NOT_COMMITTED |
| OBSERVER_UNKNOWN | client lacks a trustworthy actual-state result, independent of kernel cap custody |

**Anti-ABA / privacy:** receipt identities cannot rebind old published slot+generation/nonce to another operation as if it were the original, including after ACK/GC. Query access must not require the already MOVEd source handle or leak another Task's Endpoint/cap/count/rights by guessing a nonce. Historical COMMITTED_KI never duplicates or resurrects CapGrants another Thread already closed or revoker invalidated. Every receipt record/tombstone and pending message has bounded sponsor and M0-safe cleanup.

**P0 open:** bootstrap lost-slot ID, operation fingerprint and duplicate nonce race, K-S/K-I versus protected record coherent SMP linearization, revocation B and Endpoint MOVE, final receipt GC, metadata privacy and Linux guest status mapping. One E1 Endpoint owner and no DERIVE/DUPLICATE remain unchanged and Proposed; no type-rights table alteration follows from this addendum.

## 27. 2026-10-25 Monotonic Task operation admission and Receipt GC — Proposed

**Source:** [CAPIPC Finite Handle Namespace, Receipt Ledger & GC v0.1](../models/capipc-finite-handle-namespace-receipt-ledger-gc-v0.1.md). This is **research**, not accepted IPC semantics or a new syscall.

A new KR-M1 candidate separates **no re-admission** from **retained result knowledge**: for one live TaskGeneration, an operation sequence q known before K-S/K-I/K-U is admitted only when q equals protected high-water mark H+1. The admission cut both advances H and installs immutable PENDING record with reserved quota. A later q≤H can consult retained protected status or return EXPIRED_EVIDENCE/UNKNOWN after final GC, but **must never perform a second irreversible mutation**. Query PENDING still cannot authorize retry; only FINAL_NOT_COMMITTED proves that precise operation cannot later commit.

**P0 OPEN:** strict q admission serializes entry order and causes multi-Thread out-of-order/gap liveness difficulties; bounded window KR-M2 is a more complex comparator. Neither replaces actual K-S/K-I receipt linearization, queued custody, ReceiveClaim exposure rules, ServiceRequestId idempotency or finite sponsor/Q limits. The old KR-A1/A2 proposal remains a comparator, not superseded.

## 28. 2026-10-25 Ordered and bounded-window receipt admission study — Proposed

**Source:** [CAPIPC Ordered vs Sliding-Window Receipt Admission v0.1](../models/capipc-ordered-vs-sliding-window-receipt-admission-v0.1.md). Independent review ranks KR-M2 as multi-Thread **research priority**, not Accepted implementation. KR-M1 strict per-Task H=last contiguous consumed q is simpler; KR-M2 accepts C<q≤C+W if the protected bitmap position is unused and mandatory R[q] capacity is pre-reserved. The protected C frontier only advances across an actual atomic admission A(q) or explicit SKIP X(q), never automatically past a missing q. PENDING R[q] may remain live after C advances; final GC does not remove the replay fence.

A(q), X(q), K-S, K-I and optional K-U remain separately ordered protected cuts. A lost return cannot make admitted work uncommitted, X(q) cannot retroactively cancel a committed operation, and q admission is **not** E1 Endpoint FIFO or back-end service-effect ordering. Bound W and Q independently; permanent holes eventually block bounded-window progress unless explicitly settled. No numerical wire values, proof, tests or OS code selected.

## 29. 2026-10-25 KR-M2 cross-RFC safety-case handoff — Proposed

**Independent design review:** [CAPIPC Cross-RFC Safety Case v0.1](../reviews/capipc-cross-rfc-safety-case-v0.1.md). KR-M2 C means **contiguous consumed operation identity**, NOT completion, receipt acknowledgement, release of pending R[q], queue cleanup, RemoteEffect or durability. SEND K-S, ReceiveClaim first possible user exposure V, receive K-I, optional escrow K-U and explicit SKIP X(q) are distinct protected cuts. A(q) does not allow a later commit using stale Endpoint owner or revoked CapGrant without an explicitly proved admitted-inflight ordering. Unknown after committed MOVE cannot become false NOT_COMMITTED on result-copyout loss. All joint proof obligations remain OPEN; no E1 rights relaxation, tests, implementation or ABI approval.

## 30. 2026-10-25 CAPIPC formal-model preparation scope — Proposed

**Reference:** [CAPIPC Formal Model Preparation v0.1](../models/capipc-formal-model-preparation-v0.1.md) proposes **future** finite-state verification (not an executable spec): Task-scoped q/C/Used/R and A/X concurrency; E1 K-S + Capability MOVE vs revoke/Endpoint move; distinct receiver q, exclusive ReceiveClaim, conservative earliest byte V and atomic K-I; separate source result loss, GC, Task death/M0. C means consumed *identity*, NOT completed SEND/receive or remote effect. A, X, K-S, V, K-I and optional K-U are independent cuts. Twenty attack schedules are **unexecuted**, and no TLA+/Alloy artifact, selected KABI or Accepted IPC profile exists.

## 31. 2026-10-25 Commit vs B/Endpoint handoff ordering — Proposed

**Source:** [CAPIPC Commit, Revocation Barrier & E1 Endpoint Ownership Ordering v0.1](../models/capipc-commit-revocation-ordering-v0.1.md). Provisional CO-S distinguishes A(q)/PENDING receipt from K-S SEND queue+whole-cap-vector MOVE. An applicable B/Endpoint owner E-M that wins *before K-S* prevents a new transfer by the old owner; K-S that wins first is historical COMMITTED_KS and not retroactively unwound. Provisional CO-E-C compares cancellation/settlement of old precommit Endpoint operations before ownership handoff against quiescence and bounded in-flight alternatives; no approach is Accepted. E1 InTransit Endpoint has no effective Task owner until authorized receive K-I. Existing `NOT_COMMITTED` means **no source loss caused by this attempted SEND**, not restoration of a Handle independently closed/MOVEd or made unusable by B. Queue FIFO is by K-S commit, not q/Thread schedule. Receive V/K-I publication remains a separate P0. No source kernel code, executable model or KABI approved.
