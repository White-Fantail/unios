# UniOS RFC-001 — Kernel Boundary & Trusted Computing Base

> **Status:** Proposed — Specification v0.2 (재검토 반영·미승인·미검증)  
> **작성 기준:** 2026-10-24 (NZDT)  
> **기준 저장소:** https://github.com/White-Fantail/unios  
> **상위 문서:** [`docs/architecture/unios-architecture-v0.2.md`](https://github.com/White-Fantail/unios/blob/main/docs/architecture/unios-architecture-v0.2.md)  
> **근거:** D1–D8 및 ADR-001/002/003/004/009, RFC-001 연구 항목  
> **범위:** 아키텍처·보안 경계·사용자 공간 분리·타당성 검증안. **커널 구현·기능 검증·구현 승인 아님. 문서의 Git 등록만 수행.**

## 0. 요약 및 제안의 효력

**권장안: Practical Capability Microkernel — Kernel Boundary Profile A.** UniOS 커널은 권한 집행과 물리적 격리에 필수적인 메커니즘만 소유한다. 파일·네트워크·드라이버·데스크톱·Linux/Windows/Darwin 의미론, 일반 애플리케이션 정책을 사용자 공간으로 배치한다. 단, 커널 외부에 있다는 사실만으로 Trusted Computing Base(TCB)에서 제외되지 않는다. 모든 신뢰성 주장은 **자산, 보안 속성(기밀성·무결성·가용성), 실행 구성**별로 구체화한다.

이 초안은 기존 *Accepted — Direction*을 상세화한 **Proposed — Specification**이다. 세부 객체 모델, syscall 번호 및 C ABI, IPC 메시지 형식, 부팅 신뢰 체인, Linux guest execution/forwarding 대안은 RFC-002/003/004/005/006/007/009/010/015/016/017/018 합동 검토 후 승인한다. RFC-001 단독으로 인터페이스를 동결하거나 구현을 시작하지 않는다.

### 0.1 RFC가 해결해야 할 질문
1. Ring 0에 반드시 존재해야 하는 기능은 무엇이며 어떤 기능은 절대로 들어가지 않아야 하는가?
2. 각 사용자 프로세스가 커널 객체·장치·다른 주소공간에 접근하는 유일한 권한 경계는 무엇인가?
3. 서비스가 신뢰 대상인지 아닌지를 어떻게 자산별로 판정하며 실제 TCB 증가를 기록할 것인가?
4. Linux ELF의 네이티브와 다른 syscall 진입이 커널을 Linux ABI에 종속시키지 않고 가능한가?
5. Pager·Supervisor·드라이버가 죽거나 악성일 때 격리, 복구, 데이터 무결성 중 무엇을 보장할 수 있는가?
6. 보안·성능·하드웨어 실증 없이 어떤 문구를 공식적으로 약속해서는 안 되는가?

### 0.2 비목표
- 커널 객체의 최종 수명/권한 회수 알고리즘 확정(RFC-002)
- IPC의 최종 와이어 포맷·상수·최적화 확정(RFC-003)
- 메모리 캐시·COW·Pager·OOM 세부 알고리즘 확정(RFC-004)
- 완전한 Linux/Windows/Darwin 바이너리 호환, 모든 장치/하드웨어 지원
- seL4 같은 수학적 검증 수준을 UniOS가 달성했다고 주장하는 것
- 보안 인증, SLA 또는 출시 일정 약속

## 1. 근거와 우선순위

| v0.2 기준 | RFC-001에 적용할 제한 |
|---|---|
| D1 범용 데스크톱 목표, 개발자 우선 | 초기에는 최소 CLI/파일·서비스 실험을 검증; 일반 사용자 배포는 별도 단계 |
| D2 x86_64 UEFI / QEMU 우선 + 실물 PC 조기 검증 | x86_64 Ring 0·페이지테이블·IRQ·PCIe 장치 권한 우선; 실물 IOMMU 검증 필수 |
| D3 Native → POSIX source → 제한 Linux CLI; Wine 병행 | 커널 syscall ABI를 Linux/NT/Darwin ABI와 분리 |
| D4 Rust-first hybrid | `no_std` Rust 우선; 진입·MMU·IRQ·FFI의 unsafe/Assembly 경계 별도 감사 |
| D5 Capability 마이크로커널 및 증거 기반 예외 | 커널의 기능 확대는 기본 불허; 계측과 ADR에 근거할 때만 심사 |
| D6 core permissive, userspace copyleft 조건부 | 외부 시스템의 구현 코드가 아니라 문서화된 메커니즘을 참고 |
| D7 일반 사용자 위협 모델, 단계적 보증 | 권한·격리·부팅/서명/사용자 데이터 보호를 처음부터 설계 |
| D8 Darwin CLI 제한 연구 | Mach/Apple ABI·프레임워크를 커널 네이티브 계약에 혼입하지 않음 |

원칙의 우선순위는 **권한 무결성 > 데이터 정확성/복구 가능성 > 구조적 단순성 > 성능 > 외부 ABI 편의성**. 성능 미달을 이유로 보안 불변식을 무효화하는 예외는 허용하지 않는다.

## 2. 계층별 책임: 권장 경계

```text
┌────────────────────────────────────────────────────────────────────┐
│ Native GUI/CLI   POSIX Ports   Linux ELF   Wine/PE   Darwin R&D     │
├────────────────────────────────────────────────────────────────────┤
│ Native SDK/libc     Personality runtimes    App sandbox adapters   │
├────────────────────────────────────────────────────────────────────┤
│ User-space services (서로 별도 보호 도메인)                          │
│  Policy/Portal   Service Registry/Launcher   VFS/Pager/Storage     │
│  Network         Display/Input/Audio         Driver hosts          │
│  Compatibility brokers / loaders            Update/Recovery       │
├──────────────────── Service IPC + capabilities ────────────────────┤
│ UniOS kernel (Ring 0): enforced mechanism only                     │
│  trap/syscall/exception  address spaces/page tables  tasks/threads │
│  scheduler/timer       capabilities/object rights    IPC/wait      │
│  IRQ delivery/ack     memory accounting             MMIO/DMA guard │
├──────────────────────────── Hardware ──────────────────────────────┤
│ x86_64 CPU/MMU/APIC  RAM  UEFI  PCIe  IOMMU  storage/network/GPU    │
└────────────────────────────────────────────────────────────────────┘
```

### 2.1 커널에 배치할 기능: MUST (제안)

| 영역 | Ring 0 책임 | 사용자 공간에 남기는 정책 |
|---|---|---|
| CPU·trap/exception | Ring 전환, 인터럽트/예외 상태 저장·복원, 안전한 사용자 복귀, 커널 panic 격리 불가능 오류 | 프로세스별 예외 정책/진단·충돌 UI |
| Task/Thread | 주소공간에 결합된 실행 콘텍스트, 생성·시작·정지/종료 권한 검사, CPU 레지스터 상태 | 프로세스 생성 정책, 작업 이름·패키지·동적 로더 |
| Scheduling | runnable 선택, preemption, 최소 timer/tick, priority/CPU budget 집행 | 앱 QoS 지정 권한 결정, 워크로드 분류 |
| MMU·물리 메모리 | 페이지테이블의 최종 변경, 권한 비트·W^X 집행, 페이지 소유/재사용 시 zeroing, TLB coherence | Pager 정책, 익명/파일 매핑 정책, 페이지 캐시·압축·스왑 |
| 커널 객체/Capability | 보유 여부·타입·권한·세대 유효성 확인; grant/transfer 원자성; 객체 생명주기 참조 보장 | 서비스 이름, 사용자 승인, 파일 ACL·guest UID/GID 해석 |
| IPC/Wait | 작은 제어 메시지 전달, 권한 핸들 전송, 이벤트 대기, 예외/fault 전달, peer death 표식 | RPC 인코딩·IDL, 요청 재시도·중복 방지, 서비스 등록 |
| IRQ/Timer | APIC/IRQ 라우팅, ack/mask, 인터럽트 객체 권한, 시간 기준 최소 제공 | 장치 인터럽트 처리, 일반 타이머 서비스 |
| 물리 장치 보호 | MMIO 범위 권한 검증, DMA 가능 장치의 IOMMU domain과 map/unmap 최소 집행, 메모리 pin·격리 검사 | PCI 열거·장치 초기화·I/O 큐/드라이버, 장치 정책 |
| 계량/쿼터 | 커널 객체·페이지·CPU 등 자원 한도 집행, 계량 대상 identity | 사용자/앱/서비스별 자원 할당 정책 |
| 부팅 최소부 | 필수 초기 상태 구성, 첫 user task와 초기 제한된 권한 인계, 예외/시스템 복구 상태의 최소 보존 | 서비스 시작 순서, 정책·인증서 검증, 업데이트/복구 운영 |

**범위 조절:** IOMMU/PCIe 설정의 어떤 코드가 Ring 0에 있어야 하는지 하드웨어 의존이며, 완전한 PCI 버스 관리자나 드라이버는 기본적으로 사용자 공간이다. 커널은 반드시 최소한의 *안전한* 하드웨어 권한 집행 기능을 가져야 하며, 일부 장치에서는 사용하지 않기로 결정할 수 있다.

### 2.2 커널 밖에 배치할 기능: SHOULD NOT ENTER Ring 0
- 경로명 해석·VFS·파일시스템·블록 캐시·메타데이터/저널·fsync 의미론
- TCP/IP·DNS·TLS·방화벽 UI·VPN 프로토콜, 일반 소켓 의미론
- USB/NVMe/네트워크/GPU/audio 장치 프로토콜 구현과 대부분의 드라이버
- GUI compositor/window manager, 폰트, 텍스트, 이미지, 동영상 파서
- 사용자·앱 계정 관리, permission prompts, file picker/portal
- POSIX, Linux syscall 의미론, Win32/NT API, Darwin Mach trap·Mach-O·ObjC/Swift 프레임워크
- 환경변수·동적 링크·패키지 설치·업데이트 다운로드·원격 네트워크 서비스

**예외 심사:** 미세 성능 개선이나 구현 편의만으로 위 목록을 커널로 이동하지 않는다. 동등한 정확성·보안 하에서 사용자 공간 내 co-location/배치 처리/공유 메모리를 먼저 검증한다.

### 2.3 사용자 공간의 명확한 영역 분리
`BootCoordinator/ResourceAuthority`, `ServiceSupervisor`, `PolicyBroker/Portal`, `Pager`, `VFS`, `StorageDriverHost`, `NetStack/NetDriver`, `Display/Input`, `CompatibilityBroker`, `App`는 기본적으로 서로 다른 주소공간 및 서로 다른 초기 Capability 집합을 갖는다. 이후 동일 프로세스 내 통합(co-location)은 해당 구성요소 간 보안 경계가 사라지는 것임을 기록하는 별도 결정이 필요하다.

서비스 **검색**은 권한 **부여**가 아니다. Service Registry는 이름 → 제공자 발견만 수행하고, 커널 객체 Capability를 무조건 발행하거나 임의로 복제하지 않는다. Policy Broker의 승인도 커널의 실제 권한 집행을 우회하지 못한다. Kernel이 정책명을 이해하도록 만들지 않는다.

## 3. TCB 정의: 단일 목록이 아닌 자산별 분석

### 3.1 세 가지 TCB 집합

- **TCB-K (Kernel Enforcement):** 실행 중 유저/커널 분리를 직접 집행하는 커널 코드, privileged assembly, 주소공간/IRQ/IOMMU 집행, 필수 커널 메타데이터 및 그 무결성. 커널이 붕괴하면 대부분의 격리 보장이 동시에 상실된다.
- **TCB-B (Boot & Authority):** UEFI/펌웨어 및 초기 로더의 신뢰 가정, 서명된 시스템 이미지 검증, 초기 권한 배포, 정책 루트 및 갱신·복구 키 체인. 시스템 시작 및 권한 생성의 무결성을 보장하는 데 필요한 집합. 하드웨어/펌웨어는 UniOS가 독자적으로 검증한 부분이 아니라 **외부 신뢰 가정**으로 표시한다.
- **TCB-S(asset, property, configuration) (Service-specific):** 특정 데이터·권한·서비스의 기밀성/무결성/가용성을 위해 반드시 정상 동작해야 하는 사용자 공간 서비스. 가령 파일 데이터의 무결성에는 VFS, 파일시스템, 드라이버, 저장 장치와 관리 경로가 포함될 수 있다. 네트워크 스택은 파일 읽기 기밀성 TCB에 자동으로 들어가지는 않지만 그 파일을 실수로 전달받거나 전체 시스템 권한을 가졌다면 포함될 수 있다.

따라서 **TCB 전체 = TCB-K ∪ 필요한 TCB-B ∪ 보호 자산과 속성에 따라 달라지는 TCB-S + 하드웨어/펌웨어 가정**으로 모델링하되, 단순하게 모든 서비스의 코드 라인 수를 더해 단일 '검증된 TCB 크기'라고 주장하지 않는다.

### 3.2 컴포넌트별 신뢰 지위와 실패 영향

| 구성요소 | Kernel TCB? | 조건부 신뢰 범위 | 탈취·장애 시 예측 |
|---|---|---|---|
| UniOS Ring 0 | 예 | 모든 실행 중 격리 경계 | 전역 보안 실패/패닉 가능 |
| Firmware/UEFI/boot loader | 아니지만 Boot trust dependency | 플랫폼·초기 이미지 무결성 | 부팅 체인 전체 침해 가능 |
| Initial Resource Authority | 사용자 공간, 최상위 authority TCB | 초기 capability 배치 및 나머지 자원 권한 | 전역 권한 위조가 아니라 **과도한 합법 grant**로 전체 정책 실패 가능 |
| Policy Broker / Portal | 사용자 공간, 정책 TCB | 실제 grant 가능한 자산·민감 권한 | 파일·장치·화면 등 허용 범위 침해 가능 |
| Service Supervisor/Registry | 사용자 공간 | 서비스 실행/재시작·연결 상태; 부여된 권한 범위 | 가용성/가짜 서비스 선택 위험, 고권한이면 보안 영향 증가 |
| VFS/Pager/Storage | 사용자 공간 | 해당 파일의 내용·메타데이터 무결성/기밀성·가용성 | 파일 손실/위조, fault 정지·서비스 장애; 커널 침해로 자동 확대하지 않음 |
| Network stack/driver | 사용자 공간 | 해당 연결/패킷/노출 데이터 | 네트워크 연결/트래픽 실패·변조 |
| GPU/Input compositor | 사용자 공간 | 화면·키 입력·클립보드 데이터 | 화면 스푸핑·키로깅 가능, GUI 사용자 데이터에 대한 TCB |
| App / Linux/Win/Darwin personality | 사용자 공간 원칙적으로 비신뢰 | 스스로 허용받은 grant 범위; 원격/guest 의미 | 자기 샌드박스 안의 코드 실행·파일 손상 가능 |
| IOMMU 없는 bus-master driver | 실질적인 global hardware trust 위험 | DMA로 접근 가능한 물리 메모리 전체 가능 | 커널/타 앱 메모리 무단 수정 위험; 격리 주장 금지 |

**중요:** 서비스가 사용자를 대신해 외부의 고위험 데이터를 파싱하고 기밀 입력에 접근한다면 사용자 공간이어도 그 데이터 경로의 TCB가 된다. '모든 user-space driver는 비신뢰'란 무조건적인 주장은 성립하지 않는다.

### 3.3 권한 그래프의 불변식
1. 신규 프로세스는 **빈 권한 세트에 가까운 상태**에서 시작하며, 실행/서비스 접속/데이터 접근에 필요한 권한만 명시적으로 전달받는다. 예외는 최소 타이머/프로세스 종료 등 안전한 자체 작업의 경우만.
2. 권한을 **만드는** 작업과 권한의 **이름을 검색**하는 작업을 분리한다.
3. 기존 객체의 권한 위임은 위임자보다 강한 권한을 만들 수 없다. 그러나 **승인된 새 객체 생성은 그 객체에 대한 신규 권한을 발생**시키므로 단순 부분집합 규칙과 구별해야 한다. 생성권한의 출처·쿼터·대상 객체 종류를 RFC-002에서 정의한다.
4. `guest root`, `Windows Administrator`, `Darwin entitlement`, `POSIX uid=0` 등은 UniOS native Capability를 자동으로 만들지 않는다.
5. 각 프로세스의 핸들은 그 프로세스 내에서만 의미 있다. 타입·권한·수명·재사용 세대에 대한 커널 검증이 필요하다.
6. 공유 메모리는 권한 이전 이후에도 유지될 수 있으며, 권한 revoke는 이미 누출/복제된 데이터나 이미 완료된 DMA를 되돌릴 수 없음을 명시한다.
7. 한 주체가 객체 생성·CPU·메모리·IPC 버퍼를 무제한 요청할 수 없고, 회계·쿼터가 커널 필수 경계에서 집행된다.

**RFC-002로 넘길 결정:** 객체 목록, handle 폭, generation/epoch 또는 별도 식별자 방식, revocation 알고리즘, transfer/drop의 원자성, 객체 ABA 및 stale handle 처리. generation 부호화도 최종 확정은 아니다.

## 4. 시스템 호출 경계 및 ABI 선택

### 4.1 세 가지 서로 다른 인터페이스

- **KABI / native kernel syscall:** Ring 3 → Ring 0, 커널 객체·메모리·스레드·IPC만 다룸. 낮은 수준의 버전이 관리되는 ABI. 외부 앱에 POSIX API로 판매하지 않는다.
- **SABI / service protocols:** 서비스 endpoint를 통해 파일·소켓·GUI·권한·장치 요청을 처리하는 사용자 공간 IPC. 스키마/feature negotiation/취소 등은 개별 프로토콜 계약.
- **SDK / native API:** Rust/C/C++ 등 앱 개발자가 사용하는 편리한 API. SDK는 KABI와 SABI를 캡슐화할 수 있으며 자체 소스/바이너리 호환 정책을 갖는다.

Linux/NT/Darwin ABI는 모두 **별도의 guest ABI**이며 KABI 번호나 errno 의미를 재사용한다고 보장하지 않는다.

### 4.2 Native KABI v0 연산군(번호·세부 시그니처 미확정)

| 연산군 | 초기 후보 | 의도적 비포함 |
|---|---|---|
| Capability | `cap_close`, `cap_derive`, `cap_query` | 사용자/그룹/ACL 결론 |
| Object/Resource | `object_create_authorized`, `resource_account_query` | 임의 물리 주소 할당, 무제한 객체 생성 |
| Tasks | `task_create`, `task_control`, `thread_create`, `thread_start`, `thread_exit`, `thread_state_authorized` | ELF 로더/서브프로세스 정책 |
| AddressSpace | `address_space_map`, `address_space_unmap`, `memory_object_create/map`, `memory_pin_authorized` | 파일 경로 기반 mmap 및 page cache 정책 |
| IPC | `endpoint_create`, `ipc_send`, `ipc_receive`, `ipc_call`, `ipc_reply` | HTTP·POSIX socket·일반 RPC 의미 |
| Wait/Synchronization | `event_signal`, `wait_many`, 최소 동기화 primitive 후보 | pthread 완전 의미 |
| Fault/Exception | `exception_bind`, `fault_resume_or_terminate_authorized` | POSIX signal/SEH/NTSTATUS/guest errno |
| IRQ/Device | `irq_bind/ack`, `device_map_authorized`, `dma_domain/map/unmap_authorized` | USB/NVMe/net/GPU 프로토콜 |
| Time/Discovery | 단조시계 읽기/ABI feature query 후보 | 달력·NTP·지역 시간 정책 |

이 표는 **API inventory**이며 실제 함수명·인자·번호를 동결하지 않는다. 초기 목표는 매우 제한된 수십 개의 강제형 operation class로 관리하며 'syscall 개수가 적다'는 것을 보안의 대리 척도로 삼지 않는다. ABI 스택 레지스터·calling convention은 RFC-009.

### 4.3 x86_64 Native 진입 계약(제안)
1. Ring 3 → Ring 0 진입은 커널 소유 아키텍처 엔트리(예: `SYSCALL`/IDT)를 통과한다. syscall·페이지 폴트·CPU 예외·IRQ의 의미와 핸들링은 분리한다. `SYSRET`의 canonical-address 제약 및 필요 시 `IRETQ` 같은 안전한 복귀 경로를 고려한다.
2. 사용자·커널 스택, GS-base/TLS, 레지스터·flags, interrupt masking, 주소공간 전환 등 CPU 보안 컨텍스트의 저장·검증·복원을 아키텍처 전용 최소 코드로 격리한다.
3. KABI dispatch 이전에 **커널이 강제하는 스레드의 실행 도메인(native / restricted guest)**을 판정한다. loader/exec가 승인하는 프로세스 ABI와 스레드의 guest-enter/exit 상태를 분리한다. guest 플래그·레지스터로 native 경로를 선택할 수 없어야 한다. 상세 상태기계는 RFC-009/010의 결정 대상.
4. 인자 개수·길이·정렬·오버플로·user pointer 접근·handle type/rights·시퀀스 등을 검증한다. 사용자 포인터는 검사 후에도 경쟁에 의해 내용이 바뀔 수 있으므로 요청마다 안전한 copy/pin/snapshot 계약이 필요하다.
5. 실패 시 입력 핸들과 메시지/메모리 상태가 어떻게 변하는지 명시하고, 커널 내부 예외 또는 타 프로세스 데이터 노출로 전파되지 않아야 한다.
6. 데이터가 크거나 동기화가 필요한 syscall은 대기/취소/시그널/프로세스 종료/서비스 사망을 안전하게 다룬다. '모든 시스템 호출이 non-blocking'이라는 선험적 가정은 금지한다.
7. ABI/version/feature negotiation이 가능하더라도 초기 버전의 안정화는 RFC-018 검증 뒤에 결정한다.

**특별 주의:** syscall 게이트의 사용자 공간 shim/vDSO는 최적화와 ABI 완충 역할을 할 수 있지만 보안 경계는 아니다. 공격자는 라이브러리를 우회한 raw CPU 진입을 시도할 수 있으므로 커널 검증을 생략할 수 없다.

### 4.4 정상 Native 요청 경로

```text
App → SDK(SABI encode) → endpoint capability 확인
    → KABI IPC → Kernel: handle·rights·buffer 검증/권한 전송
    → Server: 자체 서비스 정책 및 프로토콜 검증
    → KABI IPC reply → App
```

파일을 `read()`하거나 창을 그린다고 해서 커널에 POSIX read/window syscall이 생기는 것은 아니다. 자체 thread/mapping control 등은 SDK가 직접 KABI로 호출할 수 있다.

## 5. Linux x86_64 ELF 진입의 재평가 — Restricted vs IPC

**중요한 재검토:** v0.1은 'Task ABI mode → 별도 IPC broker'를 기본안으로 권고했다. 하지만 Fuchsia Starnix는 **Restricted Execution Mode**에서 게스트 Linux 스레드를 실행하고 syscall/exception에서 *같은 스레드의 normal-mode 사용자 공간 supervisor*로 되돌리는 구조를 공식적으로 문서화한다([S13], [S14]). 따라서 기존 경로가 최선이라는 근거는 부족하며, 아래 두 방식을 경쟁 대안으로 둔다.

### 5.1 경계 후보

| 대안 | Guest→Supervisor 전환 | 커널 책임 | TCB·비용·문제 | 제안 |
|---|---|---|---|---|
| **G-R Restricted Mode** | Guest Ring 3 → kernel → 동일 스레드 normal supervisor → guest | 실행모드/주소공간 접근 강제, register snapshot, restricted enter/exit, exception reason | 이중 실행상태·주소공간 보호·TLS/FS/GS·FPU/XSTATE·signal/fault의 복잡성. 매 syscall cross-process IPC는 필수 아님 | **우선 조사 가설** |
| **G-I Separate IPC Broker** | Guest Ring 3 → kernel → 별도 Linux broker IPC → guest | 제한 continuation token, guest mode, scoped cross-task rights | 서비스 단위 격리가 명확하지만 context switches 및 broker 권한/재시작 비용 증가 | **동등한 비교 기준선** |
| G-P Binary rewriting | loader가 syscall을 패치·트램펄린화 | 최소 syscall 특례 가능 | self-modifying/JIT/직접 syscall·signal edge에 취약 | 제한된 실험 |
| G-K Linux syscalls in kernel | kernel이 Linux API 직접 해석 | 대규모 UAPI kernel code | D3/D5와 충돌하는 TCB·유지보수 부담 | 기본 기각 |
| G-V Linux VM | Guest OS/hypervisor 경계 | 별도 가상화 코드 | 직접 바이너리 호환성과 목표가 다름 | RFC-014 |

**선택 원칙:** Linux syscall 번호, Linux errno·file descriptor·fork/signal 의미론은 Native KABI에서 제외한다. G-R의 검증을 먼저 설계하되 **G-R의 최종 채택·성능·보안 우수성은 미승인**이다.

### 5.2 G-R 보안 강제 계약 후보

~~~text
Authorized loader (sandbox grants)
  -> Normal user supervisor (trusted within its sandbox)
  -> kernel restricted_enter(thread, scoped guest state)
  -> Linux guest code (supervisor mappings and native KABI inaccessible)
  -> raw SYSCALL / fault / exception / external kick
  -> kernel restricted-exit (save state + reason)
  -> same-thread normal supervisor interprets Linux UAPI
  -> kernel restricted-enter after validated re-entry state
~~~

- **Restricted mode는 별도 CPU ring이 아니라 Ring 3에서 커널이 강제하는 보호 서브도메인**이다. 사용자 코드가 상태 변수를 바꾸거나 vDSO 호출을 우회하는 것으로 탈출할 수 없어야 한다.
- guest의 raw syscall은 번호에 관계없이 guest exit로 분류한다. normal supervisor 메모리, 제어흐름, stack, TLS/FS/GS, FPU/XSTATE, PKRU, CET, 디버그 상태 등 CPU/플랫폼별 경계를 정의·검증해야 한다. 지원하지 못하는 상태는 **미지원**으로 명시한다.
- normal supervisor는 해당 Linux sandbox의 TCB-S에 들어간다. 여러 독립 앱/계정이 공유된 supervisor 상태에서 결합되면 보호 도메인 간 TCB가 증가하므로 초기에는 sandbox별 격리부터 검토한다.
- guest page fault는 파일 Pager 통지와 Linux signal 변환 중 어느 하나로 결정되어야 하며, fault handler 자체의 재진입/메모리 부족은 별도 예약으로 다룬다.
- supervisor의 임의 메모리/스레드 변경을 허용하지 말고 해당 guest task 및 capability에 한정한 delegation을 적용한다.
- kernel restricted state token은 lifecycle, epoch, 일회성 resume, peer death/kill, bounded failure를 갖는다. 권한 철회가 이미 읽은 비밀이나 이미 실행된 I/O를 되돌리지는 않는다.

### 5.3 G-I 비교 조건

IPC broker는 guest state snapshot/continuation token만 권한을 위임받고, 다른 native 프로세스에 대한 범용 메모리·task control을 받아서는 안 된다. broker crash 시 in-flight reply를 정확히 한 번 처리했다고 가정하지 않는다. G-R과 **동일한 게스트 syscall 목록·보안 경계·호스트 CPU/장치·캐시 조건**에서 p50/p99 latency, cross-process copy, context switch, fault path, 가용성을 비교해야 한다.

### 5.4 첫 Linux 연구 지원 범위

초기 대상은 x86_64 long-mode 정적 CLI ELF 및 일반 SYSCALL 진입뿐이다. 32-bit/x32, int 0x80/sysenter, vsyscall/vDSO, 동적 링커, rseq, futex, clone/fork/execve, sigreturn/신호 프레임, JIT, ptrace는 각각 범위·거부·의미론 명세 전까지 미지원이다. 이런 기능이 미지원이라고 해서 일반 Linux 바이너리 호환을 주장하지 않는다.

### 5.5 G-R 채택/기각 게이트

| 조건 | 성공을 주장하기 위한 필수 증거 |
|---|---|
| 게스트의 Native KABI 무단 사용 불가 | direct raw syscall·비정상 guest reg/PC/stack·mode-switch fuzz에서 우회 없음 |
| 보호된 supervisor 상태와 앱 사이 격리 | memory mapping, multi-sandbox, FPU/TLS, exception, timeout, debug probes |
| 정상적인 thread lifecycle | preempt/kick/kill, peer crash, signal/fault nesting의 상태기계 |
| 커널 TCB 증가 타당성 | 추가 restricted kernel code·review/proof obligations·운영 비용 산정 |
| 성능 가정 | G-I와 동등 기능 및 safety baseline에서 정량 비교 |
| 실패 시 중단 | 위반된 권한·무한 대기·unrecoverable state가 확인되면 G-I 또는 호환 범위 축소 |

**근거와 한계:** Starnix는 사용자 공간 Linux ABI 실현 가능성의 좋은 사례이지 UniOS에서 그대로 재현할 수 있다는 보증은 아니다.

## 6. 메모리·Pager·부팅의 선행 의존성

### 6.1 제안: kernel-managed enforcement + user-space paging policy
- 커널은 물리 페이지와 커널 메타데이터 소유권을 추적하고 `MemoryObject`/`AddressSpace`에 대한 매핑 허가, 접근권한, zeroing, pin/unpin, TLB 및 IOMMU 동기화를 집행한다.
- MemoryObject 생성/할당에는 명시적인 `ResourceGroup` 예산이 필요하다. 정책적 reclaim/OOM victim 선정은 사용자 공간의 Memory Manager에서 주도할 수 있으나, **최종 메모리 소진 시 커널은 무한 할당과 교착 상태를 방지하는 최저 수준의 집행 수단**을 갖는다.
- 파일 backing·Pager 요청, 파일 페이지 캐시, mmap의 backing source 결정은 VFS/Pager가 담당. 커널은 `Fault(token, address, access, task_gen)` 전달, 권한 검사, 지정된 mapping 설치 또는 오류 종료를 담당한다.
- 부팅 초기 user code, critical pager, supervisor, 권한 정책 필수 최소 루프의 실행/스택/IPC/핸들테이블에는 **부팅 준비된 wired 및 reserved 메모리 예산**이 있어야 한다. Critical Pager가 자기 파일을 page-fault로 요청하여 영원히 대기하는 구조는 금지한다.
- 파일 버퍼를 공유하더라도 `fsync` 완료의 의미와 crash-consistency는 사용자 공간 저장소 계층의 책임이며, 커널의 memory barrier만으로 내구성을 보장하지 않는다.
- **재진입·데드락:** 커널의 user-pointer copy/IPC/fault 경로, Pager 자신의 코드·stack·IPC buffer에는 self-paging cycle이 없어야 한다. wait-for graph와 wired/reserved resource owner는 RFC-004/006에서 증명할 범위로 둔다.
- **OOM:** 일반 요청은 bounded failure로 거절할 수 있어야 하며 critical recovery reservation을 비신뢰 주체가 소진하지 않도록 한다. 모든 물리적 장애를 막는다고 주장하지 않는다.
- **원자성:** Fault token 취소, map/unmap, AddressSpace teardown과 서버 재시작이 교차할 때 부분 grant가 생기지 않는 상태기계가 필요하다.

### 6.2 서비스 의존 DAG의 예

```text
Firmware/verified loader
      ↓
Kernel + static boot image (in memory, pager-independent)
      ↓
Bootstrap Resource Authority / critical minimal runtime (wired)
      ↓
Service Supervisor + Memory Manager / Pager (wired resources)
      ↓
Storage Driver and VFS (device-dependent; recovery-defined)
      ↓
Native Loader, Shell, Network, Desktop, Compatibility Brokers
```

이 그래프는 **제안된 부팅 순서일 뿐, 각 서비스를 반드시 전역 TCB로 만들라는 뜻은 아니다.** Pager와 저장소의 교차 의존성, user-space driver의 펌웨어/IO 리셋, 이미지 검증 선행 여부는 RFC-004/006/008/017에서 연결한다.

### 6.3 고장 시 필수 동작
- Critical Pager 사망: faulting thread는 무한 대기할 수 없어야 한다. bounded wait/감지/격리/재시작 또는 task 종료 정책이 필요. 복구에 필요한 메모리·코드는 pager 없이 접근 가능해야 한다.
- Memory exhaustion: supervisor에게 통지 및 임계 예약 영역 보호, 자원 그룹 단위 fail/kill 후보; 커널 메타데이터 소진으로 권한 우회나 전역 deadlock이 발생하지 않도록 설계.
- Driver crash: IRQ mask 및 DMA quiesce/reset/rebind가 검증되지 않았다면 서비스만 재시작해도 안전하다고 선언하지 않음.
- VFS/Storage crash: 열린 파일 핸들과 in-flight I/O가 다른 incarnation으로 자동 재연결되거나 exactly-once 처리된다고 가정하지 않음; 명확한 오류와 retry idempotency 요구.
- Supervisor crash: 커널은 supervisor 없이 필수 CPU/MMU 자원 집행 유지; 시작·복구·로그 경로를 별도로 확보해야 함. 완전한 availability 보장 불가.

## 7. DMA·IOMMU·IRQ — device group이 실제 격리 단위

v0.1에서 개별 PCI device/function 단위로 표현한 격리 가정을 수정한다. **IOMMU가 존재하더라도 PCIe ACS/P2P·bridge·다기능 장치·interrupt remapping 문제로 단일 function 격리가 불가능할 수 있다**([S16] Linux VFIO). 따라서 하드웨어에서 **실제로 분리 가능한 device group**을 권한 부여 단위로 검토한다.

### 7.1 필수 기계적 강제 경계

1. 펌웨어/ACPI 등 플랫폼 정보의 출처와 신뢰 한계를 기록하고 PCI 토폴로지 및 IOMMU group을 계산·검증한다. Firmware나 SMM이 임의 RAM 접근을 할 수 있다는 플랫폼 외부 신뢰 가정도 남긴다.
2. 버스 마스터 DMA와 임의 MMIO·IRQ 권한은 기본 거부한다. 올바른 IOMMU domain이 준비되지 않은 장치는 비신뢰 유저 드라이버에 할당하지 않는다.
3. buffer pin/owner 검사 → IOVA map → DMA → quiesce/ack → IOTLB invalidation completion → unmap/unpin 순서와 실패 경로를 정의한다. 재사용/회수 중인 buffer를 여전히 DMA 가능한 상태로 두지 않는다.
4. **MSI/interrupt remapping**과 APIC vector ownership을 별도로 보호한다. MSI와 peer-to-peer DMA가 우회할 수 있으면 IOVA mapping만으로는 안전하다고 하지 않는다.
5. 장치 reset/freeze 또는 DMA 정지를 증명할 수 없다면 이전 driver의 device group을 곧바로 재배분하지 않는다. fail-closed/offline/reboot 결정을 설계한다.
6. QEMU/VirtIO 환경은 CPU·IPC 기능 검증용이며 **실물 Reference PC의 IOMMU·ACS·interrupt remap·device reset 검증을 대체하지 못한다.**

### 7.2 보안/하드웨어 프로파일별 예외

| Profile | 용도 | DMA 예외 취급 | 공식 보증 수준 |
|---|---|---|---|
| LAB | 연구 VM 및 제한된 물리 실험 | trusted device 예외 허용 가능, 경고·경계 기록 | 개발 전용, 일반 사용자 격리 아님 |
| DEVELOPER PREVIEW | 선정 VM/실물 PC | 미지원 group 배제 또는 명시된 위험 프로파일 | 시험된 부품·기능만 한정 주장 |
| GENERAL-USER CANDIDATE | 향후 일반 사용자 베타 | 비신뢰 driver에는 검증된 device group·interrupt remap·DMA lifecycle 필수 | physical negative tests/부팅 신뢰 이후만 |

IOMMU 없는 PC에서 '커널 밖 드라이버이므로 안전하다'는 주장을 금지한다. 필요 시 **trusted driver 구성, 특정 장치 미지원, 범위 축소**를 따로 승인한다. GPU/USB·펌웨어 보안은 RFC-007/013/015에서 계속 검토한다.

## 8. 보안 모델 및 필수 불변식

### 8.1 위협 주체
악성 일반 앱, 손상된 Linux/Windows/Darwin compatibility 프로세스, 악성 IPC 요청·이미지/파일 파서, 손상된 user-space driver, 손상된 service broker, 공급망/업데이트 변조, 비신뢰/오작동 장치(DMA 포함), 로컬 권한 상승·자원 고갈 공격. 물리 침입·UEFI/펌웨어 손상·CPU speculation/타이밍 측면 채널은 별도 가정 및 후속 위험 평가 대상이다.

### 8.2 KB 보안 불변식 — 후보 (미승인)

불변식마다 **자산·속성·지원 HW·적용 profile·신뢰 가정·negative test**를 기록한다. 아래 SHALL은 장래 계약 제안이며 이미 실증한 속성이 아니다.

| ID | 불변식 | 미래 검증 근거 |
|---|---|---|
| KB-01 | 지원 CPU/firmware 가정 하에 Ring 3는 승인 경계 없이 privileged state를 변경할 수 없음 | MMU/SYSCALL/exception/return validation |
| KB-02 | 객체 핸들 접근은 소유·타입·권한·수명·경쟁을 검사 | forged/stale/ABA/move tests |
| KB-03 | 다른 주체의 주소공간·물리 page/device에 무권한 map/modify 불가 | negative address map tests |
| KB-04 | 기존 객체 권한의 위임은 증폭 불가; 신규 객체 권한은 별도 create authority와 자원 예산 필요 | rights lattice + creation model |
| KB-05 | IPC handle transfer는 실패·취소·peer death의 atomicity를 명시적으로 충족 | partial transfer fault injection |
| KB-06 | 일반 자원 소진은 critical recovery reserve를 침범하지 않는 통제된 오류 경로로 처리 | quota/OOM/fault storm tests |
| KB-07 | 필수 Pager/부팅 경로의 자체 의존 cycle을 방지 | wait-for graph 및 pager kill tests |
| KB-08 | Linux root/Win Admin/Darwin entitlement는 native cap이 아니며 guest raw syscall도 native KABI 우회 불가 | guest escalation probes |
| KB-09 | 격리 profile에서 group-level DMA, P2P, MSI가 허가된 범위만 사용하거나 장치가 배제됨 | physical IOMMU/ACS/remap tests |
| KB-10 | 재시작 이전 server/device incarnation의 completion/권한을 새로운 것으로 오인하지 않음 | restart/token races |
| KB-11 | 타 소유자에게 재할당한 frame/객체가 이전 내용을 노출하지 않음 | zeroing/reuse/DMA tests |
| KB-12 | debug/trace/serial/crash dump는 sandbox·개인정보 정책을 우회하는 비밀 데이터 출구가 아님 | release build audit |
| KB-13 | restricted↔normal 상태 전환 중 supervisor memory, PC/stack, TLS/CPU state의 격리가 유지됨 | FS/GS/FPU/XSTATE/PKRU/state fuzz |
| KB-14 | user pointer·공유버퍼 검증은 bounds/TOCTOU/teardown/page fault 재진입을 다룸 | malformed pointer/fault tests |
| KB-15 | bootstrap root grant와 resource authority는 출처·수명·범위를 추적하며 이름 검색만으로 권한이 생기지 않음 | authority graph review |
| KB-16 | 미검증 boot/DMA/개발 모드를 일반 사용자 보안 인증으로 표현하지 않음 | release/trust state checklist |

KB-01/09는 물리적 DMA·악성 firmware·SMM까지 전면 보호한다는 뜻이 아니다. KB-06은 모든 커널 패닉이 불가능하다는 주장이 아니라 **정의된 자원 소진 조건**에서의 bounded failure 계약이다. KB-16은 runtime enforcement가 아니라 assurance 커뮤니케이션 규칙이다.

### 8.3 보호 기능과 아직 보장하지 못하는 것
- `W^X`, ASLR, SMEP/SMAP, NX, KASLR, KPTI·CPU speculation mitigations는 첫 x86_64 Reference PC 프로파일별 지원·비용·호환성 검토 후 보장 범위를 확정한다.
- CPU 분리 및 Capability의 논리 불변식만으로 마이크로아키텍처 side channel, 장치 펌웨어, 악성 물리 접근, 서비스 가용성을 전면 방어할 수 없다.
- 서비스 재시작은 **프로세스 고립성**에 도움을 주지만, 손상 데이터 복원·완료되지 않은 I/O의 정확한 재실행·DMA 완전 중단·모든 GUI 상태 복구를 자동 보장하지 않는다.
- User-space sandbox는 설치 앱의 신뢰성 주장과 다르다. 권한 있는 compositor, picker, workspace broker의 정책 버그는 별도 위험으로 남는다.

## 9. 서비스 프로파일별 예시 권한 그래프

### 9.1 일반 GUI 앱이 문서를 선택한 경우

```text
App --(request with no path authority)--> File Portal
Portal --(user consent + scoped policy)--> VFS handle for selected file
VFS --(attenuated file endpoint cap)--> App
App --(file endpoint RPC)--> VFS/Storage driver
```

- App은 다른 파일의 이름을 추측해서 권한을 얻지 못한다.
- Portal은 승인 전에 UI 위조/대리 클릭/다른 앱으로의 임의 grant를 막아야 하는 정책 TCB다.
- VFS는 파일 데이터 무결성에 필수이며 그 기능에 필요한 저장소 드라이버도 해당 자산의 TCB에 포함된다.

### 9.2 Linux CLI 앱 — G-R 예시, 아직 미구현

~~~text
Authorized loader → Scoped normal-mode Linux supervisor
Supervisor → restricted_enter → Linux guest thread (no Native KABI)
Guest SYSCALL / exception → Ring 0 restricted-exit → same-thread supervisor
Supervisor maps guest UAPI → Native services with scoped caps
Supervisor → validated restricted re-enter → Guest continues
~~~

- Supervisor와 guest는 **서로 다른 접근권한/실행 상태**이며 겉으로 같은 process여도 암묵적으로 memory를 공유해선 안 된다.
- Guest root는 승인된 workspace/sandbox 범위 안의 가상 신원으로 취급한다. 공유 supervisor가 타 앱 데이터를 읽을 수 있으면 그 전체가 TCB가 된다.
- G-I out-of-process broker를 동일한 실험 조건의 비교 대안으로 유지한다.

### 9.3 NVMe 드라이버 및 DMA

```text
Device Authority → DriverHost [isolatable device group/MMIO/IRQ caps]
Memory Manager → DriverHost [pin-eligible buffer cap]
Kernel/IOMMU enforcement → [device_domain + scoped IOVA mapping]
DriverHost → device group (user mode; interrupt-remap and reset assumptions)
```

- 드라이버 종료 시 IOMMU map이 남아있거나 장치가 outstanding DMA를 보유하면 재시작을 통한 격리 복구가 불완전하다.

## 10. 커널 장애·서버 장애 구분

| 문제 | 권한·무결성의 보장 가능성 | 가용성·복구 계약 |
|---|---|---|
| 비신뢰 앱 크래시 | 자체 권한/주소공간 안에서 격리하는 것이 목표 | 앱 종료 및 자원 회수 가능성 연구 |
| 파일시스템 서버 크래시 | 커널 객체 격리는 유지 가능; 파일 데이터 무결성은 미확정 | 파일 핸들 invalidation/재연결/저널 리커버리 정의 필요 |
| Pager deadlock | 다른 프로세스 메모리 접근 권한을 자동 획득해선 안 됨 | faulting thread 중단, supervisor timeout/kill/recovery |
| Driver DMA 오류 | IOMMU 없으면 커널 침해 가능 | isolate/reset/quiesce 또는 해당 디바이스 비지원 |
| Policy Broker 장악 | 커널 우회 없이도 가진 grant 권한 범위에서 침해 가능 | 권한 범위 축소/감사/시스템 정책 복구 |
| Linux broker 장악 | broker에게 준 cross-task authority 내에서 guest 탈출 위험 | broker 별 sandboxes, task-scoped tokens, restart 전략 |
| Kernel bug/panic | 전역 강제 경계 신뢰 실패 | 독립 복구 이미지·크래시 로그·보안 부팅 차기 단계 |
| 악성/변조된 boot image | 초기 authority와 커널 내용 변조 가능 | verified chain 및 image manifest 범위 설정 |

### 10.1 요청 원자성·중복 문제
Kernel IPC가 메시지를 정확히 한 번 **전송**하더라도 스토리지 쓰기·주문 같은 상위 작업의 exactly-once **부작용**은 보장되지 않는다. 서비스 요청 ID, 서비스 incarnation, completed/unknown 상태, transaction/retry 정책은 SABI와 개별 서비스 계약(RFC-003/008)에 속한다. Kernel은 최소 전송·대기·취소 상태만 정확히 집행한다.

## 11. 부팅/권한 루트의 TCB 축소 연구

1. UEFI loader는 서명/무결성 검증된 kernel + 초기 이미지 + 권한/부팅 manifest를 전달하는 구성을 연구한다. 펌웨어 Secure Boot가 OS 내부의 모든 사용자 공간 서비스/업데이트까지 자동으로 검증해주지는 않는다.
2. **Root bootstrap**은 필수 Resource Authority에 한정된 초깃값(phys memory extents, device/resource descriptions, first task, boot manifest hash 등)을 전달한다.
3. Resource Authority는 서비스별 최소 grant를 선별하고 초기 broad capability를 차폐/권한 축소하거나 수명 제한하는 설계를 우선 검토한다. 그러나 런타임 자원 추가/복구에 root 권한이 필요할 수 있으므로 **'모든 root capability가 사라진다'고 약속하지 않는다.**
4. Update verifier/anti-rollback/키 관리는 임의 앱·일반 Supervisor와 분리한다. 제품 단계에서 필요한 세부 계약은 RFC-017.
5. `debug build`, unsigned boot, IOMMU disabled, trusted driver exception을 가진 구성은 일반 사용자 보안 프로파일과 다른 trust state로 식별한다.

## 12. 대안 비교 및 잠정 선택

| 안 | 커널 내용 | 사용자 공간 | 장점 | 치명적 부담 | 제안 |
|---|---|---|---|---|---|
| **M0. Strict minimal microkernel** | syscall/IPC/thread/MMU/IRQ만 극도로 최소화 | 거의 모든 policy·memory allocator·device | 작은 Ring 0, 엄격한 분리 | 초기 root task 복잡성, pager/DMA/데스크톱 복구 난도 | **비교 기준선** |
| **M1. Practical capability microkernel** | A + 안전한 kernel object/resource accounting, 최소 memory/IRQ/IOMMU enforcement, fault/compat trap mediation | 파일/네트워크/GUI/런타임/일반 드라이버 | 개발 난도와 격리 목표의 균형 | Ring 0 표면이 A보다 큼; 정확한 TCB 심사 필수 | **권장 — Kernel Boundary Profile A** |
| **M2. Early hybrid** | 일부 VFS/캐시/네트워크/스토리지 드라이버 조기 편입 | 다른 서비스 | IPC 우회 가능 | 독립 ABI 확장·보안/유지보수 부채, 실측 없는 선행 최적화 | **기본 보류** |
| **M3. 기존 커널 사용** | Linux/XNU/NT/BSD 등에 의존 | 임의 | 드라이버/ABI 편의 | 독립 kernel 목표 D1/ADR-001 위배 | **현재 목표에 부적합** |

### 12.1 하이브리드 예외 승인 게이트
동일 기능·안전성·데이터 내구성 실험 → 병목 재현 → IPC batching/shared Memory → service co-location 등 사용자 공간 최적화 → 실물 PC 비교 → 권한/TCB 증가 분석 → 독립 ADR의 명시적 승인. **서비스 장애 복구·공급망 위험까지 포함한 순이익**이 설명되지 않으면 편입하지 않는다.

### 12.2 아직 선택하지 않은 자원 모델

seL4의 untyped/retype 기반 명시적 자원 소유와 Zircon의 process/job quota 기반 객체 생성은 서로 다른 장단점이 있다([S17], [S18]). 초기 v0.1은 ResourceGroup과 object_create_authorized를 함께 나열했지만 **이를 최종 API로 확정한 것이 아니다**. RFC-002에서 다음을 평가한다.

| 후보 | 장점 | 문제/검증 |
|---|---|---|
| untyped/retype + capability-owned memory | 할당 권한/출처 명확, 명시적 회수 | 관리·단편화·부팅 복잡성 |
| kernel object table + job/resource quotas | 편리한 앱 생성·회계 | kernel allocator·OOM·ambient authority |
| hybrid | 유연성 | 중복 회계, TCB·상태 복잡화 |

**정리:** syscall 수 또는 Ring 0 LOC가 작다는 사실만으로 최소 TCB가 보장되지 않는다. 신뢰 가능한 최종 범위는 하드웨어·boot/authority·필수 서비스의 **자산별 TCB closure**로 평가한다.

## 13. Rust-first 구현 정책에 대한 설계 제한(구현 아님)

- 신규 커널/기본 user server: Rust 우선. `unsafe`는 페이지테이블/장치 레지스터/컨텍스트 전환/FFI/커널 포인터 등에 한정된 심사 대상.
- CPU별 assembly/trap/context switching은 최소 모듈, 호출 전후 레지스터·ABI·메모리 불변식 명시.
- Ring 0에는 임의 C/C++/third-party 고수준 dependency를 넣지 않는다. 외부 드라이버/호환성 라이브러리는 승인된 user space 경계에서 별도 라이선스·위협 검토.
- Rust panic·unwind, stack overflow, alloc failure, preemption/interrupt locking, SMP race, `no_std`, allocator 속성은 RFC-015 검증 대상.
- Rust를 사용했다는 사실은 unsafe/logic/hardware/side-channel/권한 오류에 대한 자동 보증이 아니다.

## 14. 검증 행렬: R1의 완료 정의

| Workstream | 계약·모델 산출물 | 통과에 필요한 미래 증거 |
|---|---|---|
| K1 Syscall entry | x86_64 native/restricted per-thread entry, 레지스터/복귀, 잘못된 사용자 포인터 오류 계약 | syscall fuzz + register-preservation/negative tests, ring transition validation |
| K2 Capabilities | 위임/권한 감소/handle generation 및 객체 소멸 상태 모델 | property tests + model checking + stale-handle races |
| K3 Memory | MemoryObject·mapping/zero/pin/OOM fault state machine | page isolation + teardown + SMP/TLB coherency tests |
| K4 IPC | handle ownership/transfer 원자성, 취소/peer death 계약 | hostile sender, partial-send, process kill stress tests |
| K5 Scheduler | block/wake/IRQ, quota와 starvation 경계 | mixed load / priority inversion / timer validation |
| K6 Boot/Pager | critical wired boot-resource DAG, fault-loop escape | kill pager, OOM, replay startup faults without kernel corruption |
| K7 Device | group ownership, ACS/P2P, DMA/MSI remapping, reset/rebind | QEMU + 실제 reference PC negative tests |
| K8 Trust Boundary | per-asset CIA-TCB, trust states, privilege graph | policy abuse tests + root/grant/sandbox auditing |
| K9 Linux guest | G-R restricted vs G-I IPC 상태기계/TCB/비용 비교, signal/fork/futex blocking list | static ELF limited-CLI proof of concept **별도 구현 승인 후** |
| K10 Performance | x86_64 VM/reference PC workload·tail-latency 측정 절차 | 동일 safety baseline, p50/p99 IPC/system service workload 비교 |

**R1 Exit Gate (설계만):** (a) kernel/server 책임 충돌 없음, (b) 전역 root/신규 생성 authority의 출처·수명 파악, (c) G-R/G-I 채택·기각 조건, (d) wired boot/pager와 group-level DMA/MSI 안전 실패 경로, (e) KB-01~16에 대한 향후 검증 방법·하드웨어 가정, (f) RFC-002/003/004/006/007/009/010/015 합동 리뷰에서 모순 해소. **현재 미통과**. 실제 보안/성능 *Validated*는 향후 구현·테스트 뒤에만 가능하다.

## 15. 열린 쟁점 및 후속 RFC 위임

| ID | 우선순위 | 질문 | 권장 후속 위치 |
|---|---|---|---|
| O-01 | P0 | 유저 task가 새 object를 만드는 권한의 뿌리와 ResourceGroup 회계 단위는? | RFC-002/004 |
| O-02 | P0 | `cap_revoke` 중 shared data, in-flight IPC, outstanding DMA 안전성은? | RFC-002/003/007 |
| O-03 | P0 | faulting task–Pager 사이의 deadlock-free bootstrap과 메모리 reserve 경계는? | RFC-004/006 |
| O-04 | P0 | 하나의 x86_64 SYSCALL에서 G-R restricted/normal과 G-I broker의 상태기계·비용·권한은? | RFC-009/010 |
| O-05 | P0 | native / guest broker에 허용되는 cross-task memory and thread control 최소 권한은? | RFC-002/010/016 |
| O-06 | P0 | UEFI→loader→kernel→초기 서비스 image 서명·롤백과 root key 경계는? | RFC-006/017 |
| O-07 | P0 | reference PC에서 IOMMU groups·ACS/P2P·MSI remapping·reset isolation이 가능한가? | RFC-007 + HW 조사 |
| O-08 | P1 | `SchedulerContext`, donation, synchronous IPC block과 starvation 회피는? | RFC-003/005 |
| O-09 | P1 | native syscall ABI를 vdso와 직접 CPU 진입 중 무엇으로 공개할 것인가? | RFC-009/018 |
| O-10 | P1 | root authority를 축소하되 동적 서비스 복구·드라이버 추가는 어떻게 허용하는가? | RFC-002/006/016 |
| O-11 | P1 | compositor/keyboard broker와 File Portal의 실제 사용자 보안 TCB는 어디까지인가? | RFC-013/016 |
| O-12 | P1 | 구체적 kernel LOC/TCB surface 계량·보안 검증 계획은? | RFC-015 |
| O-13 | P2 | 32-bit Linux/x32/NT/Darwin entry를 언제 별도 지원할지? | RFC-010/011/012 |
| O-14 | P0 | seL4-style untyped/retype와 Zircon-style job/resource-quota 중 어떤 자원 모델을 선택할 것인가? | RFC-002/004 |
| O-15 | P0 | restricted↔normal 전환 중 CPU state, memory map, fault nesting의 정확성은? | RFC-009/010/015 |
| O-16 | P0 | IOMMU group 격리가 없는 Reference PC는 어떤 profile에서 허용 가능한가? | RFC-007/015 |
| O-17 | P1 | debug/trace/panic dump에 앱 비밀이 노출되지 않도록 할 방법은? | RFC-006/015 |

## 16. 의사결정 제안 및 완료 상태

아래 모두 **Proposed — Specification**이며 D1–D8처럼 승인된 결정이 아니다.

1. **KB-D01:** Practical Capability Microkernel Profile A를 현재 경계의 우선 가설로 유지.
2. **KB-D02:** Ring 0은 resource/protection enforcement, user-space는 파일·네트워크·GUI·guest 의미론.
3. **KB-D03:** Kernel TCB/Bootstrap Authority/자산별 Service TCB와 하드웨어 신뢰 가정 분리.
4. **KB-D04:** KABI/SABI/SDK/Guest ABI 계약 분리. Raw CPU entry도 커널 강제 검증.
5. **KB-D05 [v0.2 변경]:** Linux G-R restricted mode를 우선 연구, G-I out-of-process IPC broker를 기준선으로 비교. 어느 쪽도 구현 확정 아님.
6. **KB-D06 [강화]:** critical wired/예약 메모리, device group·IOMMU·interrupt remap·reset을 필요한 HW 경계 후보로 선언.
7. **KB-D07:** hybrid 예외는 측정·TCB 영향·회복성 검증·별도 ADR 없이는 채택 금지.
8. **KB-D08:** 객체·수명·권한 회수·native syscall·restricted-state ABI의 조기 동결 금지.
9. **KB-D09 [신규]:** LAB / Developer Preview / General-user Candidate 보안 프로파일과 지원 하드웨어 조건 분리.
10. **KB-D10 [신규]:** 신규 객체 생성 권한과 기존 객체 권한 위임을 별도의 안전성 규칙으로 정의.
11. **KB-D11 [신규]:** allocator/object resource model은 RFC-002의 열린 비교 사항으로 유지.

현재 **미검증/미구현/미승인**. 다음 연구는 RFC-002이며 이 문서가 자동으로 상세 사양 승인이나 구현 Go를 뜻하지 않는다.

## 17. 공식 1차 자료와 설계 적용 메모

1. UniOS Architecture v0.2, 현재 공식 통합 방향: https://github.com/White-Fantail/unios/blob/main/docs/architecture/unios-architecture-v0.2.md
2. seL4 Tutorial — Capabilities, CSpaces and delegation: https://docs.sel4.systems/Tutorials/capabilities.html
3. seL4 — What the Proofs Assume: https://sel4.systems/Verification/assumptions.html (부팅, Assembly, hardware, DMA 등 미검증 신뢰 가정의 중요성)
4. seL4 — Verified Configurations: https://docs.sel4.systems/projects/sel4/verified-configurations.html (검증 구성/속성별로 다름, IOMMU/boot 등 일부 제외)
5. Fuchsia — Zircon Kernel Concepts: https://fuchsia.dev/fuchsia-src/concepts/kernel/concepts (객체/handle/권한, IPC, service vs kernel)
6. Fuchsia — Zircon Fundamentals: https://fuchsia.dev/fuchsia-src/get-started/learn/intro/zircon (프로세스·메모리·서비스 경계)
7. Fuchsia — Zircon Pager: https://fuchsia.dev/fuchsia-src/reference/kernel_objects/pager (사용자 Pager의 요청/제공 메커니즘과 제약)
8. Fuchsia — Zircon vDSO: https://fuchsia.dev/fuchsia-src/concepts/kernel/vdso (user-space syscall wrapper도 커널 입력 검증을 대체하지 않음)
9. Genode — General Overview: https://genode.org/documentation/general-overview/index (자산별 TCB·리소스 회계·상위 parent 권한)
10. Linux kernel — Entry/Exit Handling: https://docs.kernel.org/core-api/entry.html (일반적 하드웨어 syscall/exception entry의 보안 상태 관리)
11. Linux kernel — Adding a New System Call: https://docs.kernel.org/process/adding-syscalls.html (`clone`/`exec`/`rt_sigreturn` 등 특수 복귀 경로, ABI 다양성)
12. UEFI 2.11 Boot Manager / Secure Boot: https://uefi.org/specs/UEFI/2.11/03_Boot_Manager.html (펌웨어 이미지 검증과 OS 전 신뢰 사슬 구별)
13. **[S13]** Fuchsia Starnix syscall execution: https://fuchsia.googlesource.com/fuchsia.git/+/HEAD/docs/concepts/starnix/syscalls.md
14. **[S14]** Fuchsia RFC-0261 Restricted Execution: https://fuchsia.googlesource.com/fuchsia.git/+/HEAD/docs/contribute/governance/rfcs/0261_fast_and_efficient_user_space_kernel_emulation.md
15. **[S15]** Fuchsia RFC-0082 Starnix Alternatives: https://fuchsia.googlesource.com/fuchsia.git/+/HEAD/docs/contribute/governance/rfcs/0082_starnix.md
16. **[S16]** Linux kernel VFIO / IOMMU groups: https://docs.kernel.org/driver-api/vfio.html
17. **[S17]** seL4 Untyped Memory Model: https://docs.sel4.systems/Tutorials/untyped.html
18. **[S18]** Zircon Kernel Concepts: https://fuchsia.dev/fuchsia-src/concepts/kernel/concepts
19. **[S19]** seL4 Verification Assumptions: https://sel4.systems/Verification/assumptions.html

**연구 해석 주의:** UniOS의 restricted-mode 후보, IPC broker 대안, IOMMU-group 제약, Kernel Boundary Profile A 및 자산별 TCB 모델은 독자적인 **검토 제안**이다. seL4/Fuchsia/Linux의 소스·형식검증·성능 결과를 UniOS에 전이하지 않는다. 2026-10-24 기준 확인.

---

## 18. 심층 재검토 결과와 남은 반론

| 발견 | v0.1의 한계 | v0.2 보정 |
|---|---|---|
| Linux guest | 별도 broker를 사실상 유일한 유력안으로 기술 | Fuchsia 사례를 통해 same-thread G-R와 G-I의 명시적 경쟁 설계 |
| ABI 경계 | Process-wide ABI mode만으로 보안 분기 설명 | per-thread restricted state와 native escape 금지 계약 추가 |
| 장치 격리 | function별 DMA cap에 치우침 | IOMMU group·ACS·P2P·MSI/remap·reset 전부 검토 |
| Capability 규칙 | 모든 신규 grant를 부모 subset으로 단순화 | 기존 객체 위임과 새 객체 생성의 authority 분리 |
| 리소스 회계 | ResourceGroup 모델을 사실상 고정할 우려 | untyped/retype vs job quota를 RFC-002 대안으로 유지 |
| 보안 불변식 | 물리 HW 부재와 panic 0건 같은 절대 보장 우려 | 프로파일·신뢰 가정·bounded failure 명시 |
| 운영상 비밀 | debug/tracing/crash dump 위험 누락 | KB-12·O-17 추가 |
| 최종 의사결정 | Profile A와 Linux guest 선택이 승인된 것처럼 읽힐 소지 | accepted direction과 proposed detailed spec 분리 |

### 18.1 반대 논거를 배제하지 않는 조건부 권고

- **Strict minimal kernel이 더 낫다는 반론:** 커널 TCB는 작아져도 부팅·Pager·권한 root의 전체 TCB가 커질 수 있다. Strict minimal은 비교 기준선으로 유지한다.
- **Restricted mode는 kernel을 복잡하게 만든다는 반론:** 맞다. Ring 0의 mode switching 코드를 늘리고 supervisor의 권한도 위험이 된다. 동등 격리/성능 증거가 없다면 기존 G-I로 전환하거나 Linux 지원 범위를 줄인다.
- **사용자 공간 driver면 안전하다는 반론:** 반례는 DMA·P2P·MSI. Reference PC에서 격리 실패 시 일반 사용자 프로파일 지원을 보류한다.
- **일찍 ABI를 동결해야 SDK가 빨리 나온다는 반론:** 초기 SDK는 versioned experimental surface로 관리하고 ABI v1은 RFC-018 및 런타임·툴체인 실증 이후만 선언한다.
- **Rust면 TCB 안전하다는 반론:** unsafe/Assembly, HW, 실행 의미론·외부 C/C++과 연결되는 컴포넌트는 별도 위협 분석을 받는다.

**연구 우선순위:** RFC-002 capabilities/objects/resources → RFC-003/004/006 pager/IPC/boot 합동 검토 → RFC-007 IOMMU 실물 검토 → RFC-009/010 G-R·G-I 설계 비교 → RFC-015 security gates. 실제 구현은 별도의 Go/No-Go 승인 전까지 진행하지 않는다.

---

**문서 이력:** v0.1(2026-10-24)은 대화 첨부의 초기 연구 버전으로 보존. v0.2(2026-10-24)는 1차 공식 자료와 반대 설계안을 추가 검증하여 다듬은 **Git 등록용 Proposed Specification**. 승인·구현·실측 없음.

## 19. 2026-10-26 Boot/diagnostic L1 scope before full Native KABI — Proposed

[UniOS Minimal Boot, Exception & Memory MVP Contract v0.1](../contracts/unios-minimal-boot-exception-memory-mvp-contract-v0.1.md) separates *initial experimental Tier0A ring-0 boot/exception diagnostics* from RFC-001's eventual Task/CapGrant/KABI/Linux guest and asset-level TCB. L1 proposes only firmware handoff, private fixed privileged mappings and fail-stop exceptions/COM1; no user Tasks, IPC/Grant syscalls, driver DMA, general Pager or stable ABI. Distinct UEFI loader and freestanding Rust kernel entry/platform CPU assumptions remain OPEN, and all RFC-001 security gates remain Proposed. [Readiness Gate](../reviews/unios-implementation-entry-readiness-gate-v0.1.md) records L1 NO-GO pending further entry microcontract and explicit user Go. No code, tests, accepted ABI or formal proof.

## 20. 2026-10-26 L1 ELF/UEFI to freestanding kernel entry cut — Proposed

[L1 UEFI Handoff & Kernel Entry Microcontract v0.1](../contracts/unios-l1-uefi-handoff-kernel-entry-microcontract-v0.1.md) isolates UEFI PE32+ loader from freestanding Rust ELF kernel entry, with deliberately bounded boot ELF parsing, checked reserved BootContext, post-ExitBootServices no-firmware path, a provisional high-half CR3 + mapped trampoline, and reviewed UEFI→SysV/stack/no-red-zone CPU state. This is a **single-BSP DevUnsigned Tier0A experimental TCB**, not a user Task/Capability/SABI kernel or production verified boot. H-O01/02/03 still block L1 readiness. No Accepted ABI, code, model, VM boot or implementation authorization.

## 21. 2026-10-26 L1 fixed-link/CR3/entry scope is ready to request lab implementation Go — Proposed

[L1 Fixed-Link ELF, CR3 Bridge & Entry-State Closure v0.1](../contracts/unios-l1-fixed-link-elf-cr3-bridge-entry-state-closure-v0.1.md) selects **lab-only** fixed higher-half ELF64 ET_EXEC with bounded loader, matched bridge linear mappings across CR3, early GDT/IDT/TSS/IST and UEFI-to-freestanding SysV CPU ABI. The [L1 Go/No-Go Review](../reviews/unios-l1-implementation-go-no-go-assessment-v0.1.md) finds **experimental QEMU boot design READY TO REQUEST USER GO**, not authorized or validated. This excludes user Tasks, Capability/IPC/VM Pager, real hardware/driver DMA, general kernel security and public Native ABI; prior RFC-001 CAPIPC/TCB P0 gates are not implicitly resolved. No source/model/test has run.
