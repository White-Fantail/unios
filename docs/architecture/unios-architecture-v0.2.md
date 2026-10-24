# UniOS — 통합 아키텍처 및 기술 연구 설계서 v0.2

> **문서 상태:** Official Architecture Draft v0.2 — Accepted Direction / Detailed Specifications Pending  
> **결정 기준일:** 2026-10-24 (NZDT)  
> **저장소:** https://github.com/White-Fantail/unios  
> **선행 문서:** [5대 커널 비교 및 독립 운영체제 기술 설계서 v0.1](kernel-architecture-and-feasibility-v0.1.md)  
> **범위:** 아키텍처·기술 타당성·연구 로드맵. **구현 계획의 자동 승인 또는 동작 검증을 뜻하지 않음.**

## 0. 문서의 효력과 용어

이 문서는 UniOS 프로젝트의 **현재 공식 통합 아키텍처 초안**이다. v0.1을 삭제·수정하지 않고 연구 이력으로 보존한다. 2026-10-24에 승인된 **여덟 가지 상위 방향**을 통합하며, v0.1의 권고안 가운데 상위 결정과 충돌하지 않는 기술적 근거·비교는 계속 참조한다.

상태는 엄격히 구분한다.

| 상태 | 의미 |
|---|---|
| **Accepted — Direction** | 상위 제품·아키텍처 원칙이 승인되었음. 아직 구현·실측으로 검증되지 않음 |
| **Proposed — Specification** | 기능의 상세 API, 알고리즘, 정책, 비용과 측정 기준은 제안 상태 |
| **Validated** | 문서화된 환경·반복 가능한 테스트에서 증거를 확보한 상태. **현재 해당 없음** |
| **Implemented** | 실제 코드 구현 완료. **현재 해당 없음** |

**현재 상태:** 연구·설계 전용. UniOS 커널, 드라이버, 호환성 계층, Native 앱 런타임은 구현되어 있지 않다. 운영체제 ISO, 지원 앱, 실물 PC 사용성, 보안 성능에 대한 보증도 없다.

독립 커널은 Linux/BSD/XNU/NT 커널의 포크가 아닌 UniOS 자체 privileged kernel과 ABI·객체·권한 모델을 의미한다. UEFI·PCIe·ELF 같은 공개 표준과 적법한 사용자 공간 라이브러리의 활용은 독립성 상실이 아니다.

호환성은 반드시 세 범주로 구별한다. **Source port**는 UniOS용 재컴파일, **binary compatibility**는 원래 플랫폼용 바이너리의 수정 없는 실행, **guest virtualization**은 다른 운영체제 자체의 가상 실행이다. 어느 하나의 성과가 다른 방식의 성공을 뜻하지 않는다.

## 1. 제품 목표와 범위

**Vision:** UniOS는 독립적인 Rust 중심 Capability 커널에 Native 데스크톱 환경을 구축하고, Linux·Windows·Darwin 소프트웨어와의 상호운용성을 검증 가능한 범위에서 단계적으로 확대하는 범용 데스크톱 OS를 지향한다.

- **최종 사용자 가치:** 일상적으로 사용할 수 있는 범용 데스크톱 운영체제.
- **최초 사용자:** 개발자 및 기술 애호가. 제한된 VM·하드웨어와 기본 개발 환경으로 검증.
- **자체 제품 가치:** Native SDK, 터미널, 파일 관리, 기본 그래픽·입력·저장소, 안전한 앱 격리. 다른 OS 호환성이 완성되지 않아도 독립적인 가치를 지녀야 한다.
- **현재 비목표:** 즉각적인 Windows/Linux/macOS 앱 전체 호환, 범용 GPU/Wi-Fi/노트북 지원, Windows/macOS 드라이버 로딩, 모든 Docker 기능, 게임/DRM/안티치트 호환, 상용 출시.
- **기술 우선 원칙:** 정확성·데이터 보존·보안 불변식이 IPC·처리량·GUI 지연 최적화보다 우선한다.

사용자 가치 검증의 제품 단계를 Prototype → Developer Preview → Compatibility Preview → Reference PC Hardware Alpha → General Desktop Beta → Extended Compatibility로 가정한다. 이는 **출시 일정이나 개발 승인**이 아닌 연구용 목표 단계다. 단계별 일부 검증은 병행·재배열될 수 있다.

## 2. 승인된 여덟 가지 상위 결정

| ID | 결정 | Accepted — Direction | 주요 미해결 사항 |
|---|---|---|---|
| D1 | 사용자·제품 | **범용 데스크톱 목표 + 개발자 우선** | 첫 Developer Preview의 지원 워크플로 |
| D2 | 최초 하드웨어 | **x86_64 UEFI, QEMU/OVMF/VirtIO VM 우선 + 특정 실물 PC 조기 검증** | 정확한 Reference PC 모델·장치 목록 |
| D3 | 앱 호환 우선순위 | **Native SDK → POSIX 소스 포팅 → 제한적 Linux x86_64 CLI 바이너리**; Wine/Windows 의존성 연구 병행 | Linux syscall 진입·전달, libc, 앱별 커버리지 |
| D4 | 구현 언어 | **Rust-first Hybrid**; 핵심 신규 커널·서버는 Rust 우선, Assembly/unsafe/C/C++은 통제된 경계에서 허용 | Rust 사용자 공간 타깃·libc·FFI 안정화 |
| D5 | 커널 형태 | **실용적 Capability 마이크로커널 우선 + 증거 기반의 제한적 하이브리드 예외** | Pager/VFS/캐시·IPC fast path의 위치 |
| D6 | 라이선스 | **Core Permissive + Userspace Copyleft 조건부 허용**; 자체 신규 핵심 코드에 MIT OR Apache-2.0 우선 검토 | 실제 소스 라이선스·기여 정책·제3자 결합 심사 |
| D7 | 보안 | **일반 사용자용 보안 아키텍처를 처음부터 설계하고 단계적으로 보증** | Secure Boot/업데이트/암호화·권한 회수의 세부 계약 |
| D8 | Darwin/macOS | **CLI·Mach-O 제한 연구를 우선, GUI/AppKit/Metal은 장기 연구**; 검증된 기능만 공식 지원 표기 | Darling 포팅·라이선스·프레임워크 한계 |

**경계:** 방향이 승인되었더라도 개별 기술·API·성능 수치·지원 제품·SDK 버전·상용 배포는 자동 승인되지 않는다.

## 3. 논리 시스템 구조

전체 구조는 여섯 책임 영역으로 분리한다.

1. **애플리케이션:** Native GUI/CLI, UniOS용으로 재컴파일한 POSIX 도구, Linux ELF 프로그램, Windows PE 프로그램, 장기 Darwin Mach-O 연구 대상.
2. **실행·호환성 런타임:** Native SDK/C ABI, UniOS libc, Linux Personality, Wine 포팅·적응 계층, 별도 Darwin Personality. 호환성은 실행 ABI만 공유하지 않으며 각 게스트의 파일·프로세스·예외·권한 의미를 구현한다.
3. **사용자 공간 서비스:** Service Supervisor/Registry, VFS·저장소, 네트워크, 그래픽·입력·오디오, Policy Broker, 대부분의 장치·드라이버 호스트.
4. **언어 독립적 계약:** Native 시스템 서비스 프로토콜·IDL, IPC 메시지 스키마, 오류·취소·버전 계약. Native syscall ABI와 상위 SDK를 분리.
5. **독립 커널/TCB:** CPU·스레드·스케줄러, 메모리 보호와 페이지 테이블의 최소 신뢰 부분, Capability 객체/핸들, IPC·동기화 기본 기능, IRQ·타이머, 권한 통제된 MMIO/DMA 중재.
6. **하드웨어:** Tier 0 QEMU/OVMF/VirtIO, Tier 1 x86_64 Reference Mini PC, 이후 제한적인 다른 PC와 ARM64 연구.

**기본적인 커널 밖 배치:** 네트워크 스택, VFS·파일시스템, Linux/Windows/Darwin 런타임, Compositor, 오디오, 일반 장치 드라이버. **아직 결정되지 않은 배치:** 파일 페이지 캐시와 사용자 Pager의 상세 경계, 커널 필수 저장소 경로, 고성능 I/O 경로, 일부 GPU/부팅 드라이버.

모든 서비스 요청이 같은 순서로 계층을 통과한다는 의미는 아니다. 최소 커널 syscall에 직접 진입하는 경로와 서비스 IPC 경로를 각각 정의해야 한다.

## 4. 커널, 객체 및 메모리 계약 — 제안·미검증

### 4.1 객체/Capability

초기 객체 후보: Task, Thread, ResourceGroup, AddressSpace, MemoryObject, ChannelEndpoint, Event/WaitSet, Interrupt, DeviceResource. 객체의 실제 목록과 상태 모델은 RFC-002에서 결정한다.

필수 불변식 후보:

- 커널 객체는 유효한 권한 핸들 없이 접근할 수 없고, 인덱스 추측으로 새 권한을 얻을 수 없다.
- 위임은 보유한 권한 범위 안에서만 가능하다. Guest Linux root/Windows Administrator/Darwin entitlement는 UniOS 관리자 Capability로 자동 승격되지 않는다.
- close/transfer/revoke와 객체 파괴는 서로 다른 작업이며, 재사용 세대·경쟁 상태와 이미 공유된 버퍼·DMA에 미치는 영향을 정의한다.
- IPC 파서·syscall 입력 검증 실패는 커널 정보 노출이나 권한 상승을 초래해서는 안 된다.
- Trusted Computing Base에는 커널뿐 아니라 부팅·권한 정책과 필수 신뢰 서비스가 포함될 수 있다. '작은 커널'과 '작은 전체 TCB'를 동일시하지 않는다.

### 4.2 IPC/스케줄링

동기식 소형 IPC, 비동기 이벤트, 공유 MemoryObject, WaitSet, 타임아웃·취소·백프레셔를 분리한다. 메시지/핸들 전송의 원자성, 요청 ID, 중복 실행 방지, peer 재시작 탐지, 스케줄링 자원·우선순위 전달을 RFC-003/005에서 명시한다.

소형 제어 메시지는 IPC, 큰 데이터는 권한 통제된 공유 버퍼가 기본 후보이지만 무조건 zero-copy를 보장하지 않는다. 서비스 co-location과 fast path를 먼저 검토한 후에도 실측 병목이 해결되지 않으면 제한적인 커널 편입을 별도 ADR로 심사한다.

### 4.3 VM/Pager/복구

커널은 주소공간 및 물리 페이지 보호를 책임지고, 파일 기반 매핑·Pager·캐시는 사용자 공간 우선으로 연구한다. 부팅 필수 코드/스택/복구 자원은 Pager의 고장·OOM 상황에서도 필요한 최소 작업을 수행할 수 있어야 한다.

필수 연구 항목: COW, 공유 메모리 수명, DMA 버퍼 pin/map, 페이지 폴트 의존성, 복구 예약 메모리, OOM 정책, 읽기 전용 시스템 이미지, 데이터 볼륨, 캐시 일관성, fsync 및 전원 중단 후 내구성.

서비스 재시작은 자동 성공을 보장하지 않는다. 인스턴스 세대와 핸들 재연결, 진행 중 I/O의 완료·중복 여부, 파일시스템 일관성, DMA 중지 및 장치 reset/rebind를 명시한다.

## 5. 첫 하드웨어와 실물 검증 전략

| Tier | 환경 | 역할 |
|---|---|---|
| **0A** | QEMU x86_64 Q35 + OVMF + VirtIO console/block/net | 커널·부팅·IPC·서비스 기준선 |
| **0B** | 0A + VirtIO GPU 2D/입력 | 데스크톱 기능 연구 |
| **0C** | QEMU의 NVMe·USB xHCI 등 비-VirtIO 장치 구성 | 장치 추상화 검증 |
| **1** | 선정한 x86_64 UEFI Mini PC 한 모델 | 초기 실물 부팅·핵심 I/O·DMA 및 장애 검증 |
| **2** | 기타 데스크톱·노트북 | 실험적 확대 |
| **3** | ARM64 및 Apple Silicon | 별도 장기 아키텍처 연구 |

**Tier 1을 GUI 완성 이후로 미루지 않는다.** Reference PC 선정 시 UEFI 설정 복구, PCI ID, NVMe·USB·유선 LAN, 화면 출력, 시리얼 또는 대체 디버깅, IOMMU·DMA 격리 가능성과 펌웨어 사용 권한을 조사한다. 처음에는 기존 OS 디스크를 변경하지 않는 별도 부팅 매체를 사용하도록 시험 계획을 설계한다.

UEFI 화면 출력과 UniOS의 영속적인 GPU/USB 지원은 다르다. USB, Wi-Fi, 절전·복원, GPU 가속, 노트북 배터리는 별도 지원 게이트를 통과해야 한다.

## 6. Native 개발 플랫폼 및 호환성

### 6.1 Native SDK

최소 커널 syscall과 사용자 공간 서비스 API를 분리하고 언어 독립적 공개 ABI를 검토한다. 기본 후보는 C ABI, opaque handle, 명시적 소유권·오류·취소 규칙과 Rust/C/C++ 바인딩이다. Rust 내부 자료구조 배치나 기본 Rust ABI를 외부에 고정하지 않는다.

초기 Rust 커널은 no_std 기반. UniOS 사용자 공간 Rust 표준 라이브러리, custom target, libc, C++ 런타임, 동적 로더는 각각 독립적인 포팅·검증 과제이다. Stable ABI v1은 실증 없이 조기에 선언하지 않는다.

### 6.2 소스 포팅 vs 바이너리 호환

**Native POSIX Source Ports**는 UniOS libc와 시스템 서비스 위에서 프로그램을 재컴파일한다. libc 후보 musl(MIT), relibc(MIT), 자체 구현을 별도 평가한다.

**Linux x86_64 CLI Binary Compatibility**는 원래 Linux용으로 빌드된 ELF 바이너리의 시작·종료·syscall·TLS·시그널·프로세스/스레드·mmap·파일·네트워크 의미를 재현하는 별도 문제다. 첫 검증은 제한된 정적 ELF 프로그램, 이후 동적 링크·glibc/musl·복수 프로세스·소켓 등으로 확대한다. **Linux syscall이 UniOS 커널에 진입한 후 Linux Personality로 제어권을 중재하는 메커니즘은 P0 미해결 과제**다.

**Windows PE/GUI**는 Wine의 UniOS 포팅 의존성을 초기부터 조사한다. PE 로더, Win32/NT 의미, GDI·창·글꼴·COM·레지스트리·그래픽의 단계적 연구가 필요하다. Wine을 그대로 사용할 수 있다고 가정하지 않는다. Windows 커널 드라이버, DRM·안티치트, 광범위한 3D 게임은 초기 비목표다.

**Darwin/macOS**는 공개 소스 CLI 포팅과 x86_64 Mach-O 바이너리 실행을 구별한다. Objective-C·Swift·Foundation·AppKit·Metal은 각기 독립 연구, Darling은 Linux 의존성·GPL·Apple APSL 조건을 검토한 후 활용 여부 결정. 일반 macOS GUI 앱 및 macOS VM 범용 PC 실행을 보장하지 않는다.

### 6.3 호환성 주장 등급

| 등급 | 검증 의미 |
|---|---|
| L0 | 파일 형식 식별 |
| L1 | 프로세스 시작·정상 종료 |
| L2 | 선정한 CLI 기능·파일·스레드·네트워크 등 실제 동작 |
| L3 | 기본 GUI·입력·클립보드 동작 |
| L4 | 설치·업데이트·오디오·GPU/인쇄 등 실무 워크플로 |
| L5 | 지정한 앱/버전/환경의 회귀 테스트 요구사항 충족 |

지원 수준은 **앱·버전·ISA·런타임 버전·하드웨어·테스트 기능·검증일** 단위로 기록한다. UniOS 전체가 Linux/Windows/macOS 호환이라고 포괄 주장하지 않는다. 현재 모든 외부 바이너리 호환성은 **미구현/미검증**이다.

## 7. 보안 모델 및 릴리스 신뢰

**보안 목표:** 일반 사용자 위협 모델을 초기부터 설계하되 내부 연구/공개 개발자/일반 사용자 배포의 보증 수준을 분리한다.

- **Kernel enforcement:** Capability 최소 권한, 메모리 분리, 검증된 핸들·IPC, W^X/ASLR 등의 플랫폼 적합 정책.
- **App Sandbox:** 일반 GUI 앱의 다른 앱 데이터 접근 기본 거부, 명시적 선택 파일에 대한 Portal, 카메라·마이크·화면 공유 권한의 제한된 수명.
- **Compatibility sandbox:** 게스트 root/Administrator/entitlement는 네이티브 권한과 분리. 개발 도구는 범위가 명시된 Workspace 권한으로 실행하는 방향 검토.
- **Device isolation:** MMIO/IRQ/DMA 허용 범위, IOMMU 지원 확인, 미지원 장치의 신뢰 예외 또는 지원 중단.
- **Boot:** UEFI Secure Boot의 EFI 이미지 검증과 커널·서비스 이미지 검증 체인을 구분. 개발 키와 제품 키 분리.
- **Updates:** Root/릴리스 키 분리, 서명·해시·버전·최신성 검증, 키 회전, A/B 슬롯 및 rollback 방지, 별도 검증된 복구 이미지. 실패 복구와 오래된 취약 버전 차단의 충돌을 설계에 명시.
- **User data:** 앱별 권한, 전체 디스크 암호화·TPM 키 정책·복구 키, 백업 및 실제 복원 검증. 디스크 암호화는 실행 중 악성 앱을 막는 수단이 아니다.
- **Supply chain:** 재현 가능한 출처/커밋·툴체인 기록, 승인된 의존성, SBOM·provenance, 릴리스 서명과 보안 보고 체계.

단계: **S0** 위협 모델·계약 → **S1** 내부 커널 격리 실험 → **S2** 공개 개발자 프리뷰의 최소 서명·Sandbox·업데이트 경로 → **S3** 일반 사용자 데스크톱 베타의 검증된 부팅·서명 업데이트·암호화·복구. 단계 통과는 테스트 증거로만 인정한다.

## 8. 라이선스 및 외부 코드 도입 정책

UniOS가 적법하게 권리를 보유한 신규 커널·핵심 SDK 코드에는 **MIT OR Apache-2.0**를 우선 검토한다. 최종 LICENSE·기여 정책은 코드 공개 전에 별도로 확정한다.

사용자 공간의 Wine(LGPL), glibc(LGPL), GNU 도구(GPL), Darling(GPL과 별도 모듈 라이선스 등)은 사용 위치·링크·수정·배포 형태에 따른 검토 후 조건부 허용. Copyleft 코드가 사용자 공간에 있다는 사실만으로 소스 제공·결합 의무가 사라지지 않는다.

Linux GPL 커널 구현 코드를 UniOS 핵심에 직접 복사하지 않는다. Linux UAPI/예외는 파일별 SPDX와 사용 조건을 확인한다. FreeBSD/BSD/MIT 드라이버도 파일별 검토가 필요하다. Apple APSL, 비공개 SDK·AppKit·Metal 바이너리, 펌웨어의 재배포는 별도 법률·권한 심사 전 금지한다.

제3자 구성요소 등록 항목: 이름, 고정 버전/Commit, 출처, SPDX, 변경 여부, 링크·IPC 관계, 배포 위치, 소스·고지 의무, 보안 상태, 승인자. LGPL 정적 링크 재링크 의무, GPLv3 제품 설치 정보 및 부팅/업데이트 서명 조건은 실제 배포 형태별로 검토한다. 이 문서는 법률 자문이 아니다.

## 9. 성능·정확성·안전성 검증 — 전부 잠정 기준

모든 수치는 **향후 Reference PC 및 통제된 비교 실험을 위한 가설**이며 UniOS 실측 결과, SLA 또는 구현 합격 선언이 아니다. QEMU TCG 결과를 실물 성능 대신 사용하지 않는다.

| 검사 | 초기 설계 예산·관측값 | 필수 비교 조건 |
|---|---|---|
| 64B 동기 IPC | 동일 코어 p50 ≤ 5 µs / p99 ≤ 20 µs 가설 | CPU/코어/보안 완화/호출 경로 고정 |
| Cross-core IPC | p99 ≤ 50 µs 가설 | CPU affinity, 스케줄링·부하 명시 |
| 소형 파일 워크로드 | 하이브리드 대안 대비 처리량 ≥ 80% 가설 | 캐시·파일시스템·정확성 동등 |
| 스토리지/네트워크 | 동등 기능의 대안 대비 ≥ 80% 가설 | 장치·DMA·CPU 비용 분리 |
| GUI | 사용자 입력 반응 p95 ≤ 50 ms 가설 | 실제 표시 시간, 2D SW/GPU 분리 |
| IPC+서비스 경계 | UI 작업당 p99 ≤ 0.8 ms 가설 | 전체 이벤트 경로 추적 |
| 드라이버/서비스 충돌 | 지정된 fault injection에서 권한 상승·예기치 않은 커널 패닉 0건 목표 | 서비스 종류·주입 범위 기록 |
| 업데이트/저장소 | 지정된 전원 차단·fsync·rollback 실험의 검증된 데이터 손실 0건 목표 | 실패 시점·블록 순서·복구 검증 |
| 공급망 | 공개 릴리스별 소스·의존성·서명 출처 100% 추적 목표 | 산출물/소스 연결 검증 |

p50/p95/p99, 최악/실패 경로, 반복 실험, 실물과 VM, warm/cold cache, 단일/다중 코어를 분리한다. 성능 비교는 같은 기능과 안전성·데이터 내구성을 갖춰야 한다.

**Hybrid exception gate:** 실제 병목 발견 → 배치·IPC 축소 → 공유 메모리/스케줄링 → 사용자 서비스 co-location → 그래도 미달할 경우에만 권한·TCB 증가와 성능 개선의 증거를 포함한 **별도 ADR**로 커널 편입 심사. 가정에 따른 선제적 커널 기능 확대는 지양한다.

## 10. 결정 간 충돌·위험 등록부

| 우선 | 위험 | 설계 검증 작업 | 중단/재검토 신호 |
|---|---|---|---|
| P0 | Linux ELF syscall 진입 vs Native 독립 ABI | 진입·에뮬레이션·복귀 계약과 보안 경계 | guest ABI가 커널 전체를 지배 |
| P0 | 사용자 Pager/VFS vs 부팅·복구 | 서비스 의존 DAG, wired core, OOM 및 fault 모델 | 필수 서비스 복구 교착 |
| P0 | Capability vs POSIX/Win32 권한 | guest 권한·네이티브 grant 매핑 | 임의 privileged 예외 증가 |
| P0 | 사용자 드라이버 vs DMA | IOMMU domain, pin/unmap, reset/rebind | 실기기에서 격리 불가능 |
| P0 | Secure Boot vs A/B rollback | 신뢰 버전·복구 슬롯·키 회전 | 업데이트 실패로 회복 불가능 |
| P1 | Rust-first vs libc/Wine FFI | C ABI·소유권·패닉/예외 경계 | 언어 경계에서 메모리 안전성 저하 |
| P1 | IPC/GUI 응답과 마이크로커널 | 서비스 호출 그래프와 성능 예산 | 측정 후 지속적·구조적 초과 |
| P1 | Native ABI 안정성과 진화 | 기능 탐색·프로토콜 버전·폐기 계약 | 초기 API 조기 고착 |
| P1 | 라이선스 혼합 | 링크·배포·파일별 출처 심사 | 핵심 Permissive 경계 충돌 |
| P2 | Darling·AppKit·Metal | Darwin 종속성과 법률 검토 | 초기 Native/Linux 목표 잠식 |

우선순위는 **P0 선결 조건**, **P1 상세 설계 중 병행 연구**, **P2 별도 장기 연구**를 뜻하며, 현재 위험이 발생했다고 판단한 것은 아니다.

## 11. ADR 결정 기록 정규화안

v0.1에는 ADR-001~005가 **Proposed**로 존재한다. 이번 협의에서 사용된 ADR-006~012 표기는 Git에 등록되지 않은 임시 제안 번호였다. 혼동을 막기 위해 아래 번호를 **v0.2의 정규화된 색인**으로 사용한다. 개별 ADR 파일은 추후 작성하며 **현재는 이 문서가 결정 출처**다.

| ADR | 명칭 | 근거 결정 | 현재 상태 |
|---|---|---|---|
| ADR-001 | Independent Kernel | D1 + v0.1 독립성 원칙 | Accepted — Direction |
| ADR-002 | Capability Microkernel with Evidence-based Hybrid Exceptions | D5 | Accepted — Direction |
| ADR-003 | Native ABI and Compatibility Separation | D3 | Accepted — Direction |
| ADR-004 | VM-first, Early Reference PC | D2 | Accepted — Direction |
| ADR-005 | Evidence-based Compatibility Claims | D3, D8 | Accepted — Direction |
| ADR-006 | General Desktop Vision, Developer-first Delivery | D1 | Accepted — Direction |
| ADR-007 | Rust-first Hybrid Language Policy | D4 | Accepted — Direction |
| ADR-008 | Core Permissive and Controlled Userspace Copyleft | D6 | Accepted — Direction |
| ADR-009 | Security-by-design with Staged Assurance | D7 | Accepted — Direction |
| ADR-010 | Darwin/macOS Research Scope and Claims | D8 | Accepted — Direction |

각 정식 ADR는 Context, Decision, Alternatives, Rationale, Consequences, Invariants, Validation, Revisit Conditions, Status를 기록한다. 상세 RFC의 변경이나 실패 증거에 따라 별도 검토 후 ADR를 대체할 수 있으나 과거 결정 이력을 삭제하지 않는다.

## 12. RFC 상세 연구 등록부

**기존 RFC-001~015 번호를 보존**하고 부족한 경계를 RFC-016~018 후보로 추가한다. 파일 생성·승인·실험을 완료한 목록이 아니라 앞으로 다뤄야 할 연구 주제다.

| RFC | 연구 대상 | 우선 |
|---|---|---|
| 001 | Kernel Boundary and Trusted Computing Base | P0 |
| 002 | Object, Capability, Lifetime, Delegation and Revocation | P0 |
| 003 | IPC Wire Protocol and Synchronization | P0 |
| 004 | Virtual Memory, Pager, Cache and OOM Contract | P0 |
| 005 | Scheduling, Realtime and Priority Donation | P1 |
| 006 | Boot Trust Chain, Service Startup and Recovery | P0 |
| 007 | User-space Drivers, DMA and IOMMU | P1, 초기 HW 제약은 P0 |
| 008 | VFS, Namespaces and Crash-consistent Storage | P1, 부팅 의존은 P0 |
| 009 | Native ABI, SDK, Loader and Toolchain | P0 |
| 010 | Linux Personality Syscall Mapping and Entry Mechanism | P0 |
| 011 | Windows Compatibility / Wine Dependencies | P1 분석 병행 |
| 012 | Darwin/Mach-O Feasibility and Legal Review | P2 |
| 013 | Graphics, Audio, Desktop and GPU | P1 |
| 014 | Hypervisor / Guest Virtualization Feasibility | P2 |
| 015 | Threat Model, Quality Gates and Observability | P0 기준 / P1 세부 |
| 016 | Application Sandbox, Permissions and Portals | P1, 권한 원칙 P0 |
| 017 | Verified Boot, Update, Anti-rollback and Recovery | P1, 신뢰 모델 P0 |
| 018 | System ABI / Service Protocol Versioning | P1, 안정화 원칙 P0 |

선행 의존: **RFC-001+002 → RFC-003+004+006 공동 리뷰 → RFC-009+010**, 하드웨어/드라이버·보안·호환성 요구사항 분석은 초기에 병행. 개별 파일을 분리하더라도 Pager/Boot/OOM과 Update/Storage/Boot는 통합 리뷰가 필요하다.

## 13. 설계 연구 로드맵과 Go/No-Go

| Stage | 목적 | 필수 산출물 | Exit Gate |
|---|---|---|---|
| **R0** | Architecture Baseline | 본 v0.2, 승인 방향·미해결 목록·ADR 색인 | 여덟 결정이 문서로 추적 가능 |
| **R1** | Kernel Object / TCB Contracts | 객체·권한·IPC·메모리·스케줄러 명세 초안 | 주요 수명/권한 불변식 충돌 해소 |
| **R2** | Boot, Pager and Recovery | UEFI·부팅 서비스 DAG·OOM·서비스/스토리지 복구 시나리오 | 필수 서비스 교착·복구 모델 설명 |
| **R3** | Native Developer Platform | C ABI·SDK·IDL·Rust target/libc 연구 | 최소 Native CLI/GUI의 계약 정의 |
| **R4** | HW and Desktop Feasibility | VM/PC 프로파일, DMA/드라이버·그래픽·성능 실험 프로토콜 | 선정 장치의 실현 가능성 평가 |
| **R5** | Compatibility and Trust | Linux entry, Wine/Darwin 의존성, 법률·보안·업데이트/릴리스 정책 | 차단 요소·지원 범위·테스트 목록 확정 |
| **R6** | Implementation Go/No-Go | 인력·예산·기간 가정, MVP 범위, 품질 게이트, 결정 기록 | **명시적인 구현 착수 승인 또는 연구 연장/범위 축소** |

연구는 필수 의존성을 지키면서 병행 가능하다. 현재는 **R0 상위 결정 정리 단계**이며 R1~R6의 검증을 완료하지 않았다. 팀 규모와 지원 하드웨어가 확정되지 않은 상태에서 기간·성능·앱 호환성·보안 수준을 제품 약속으로 발표하지 않는다.

### 다음 설계 작업의 권장 순서

1. **RFC-001 Kernel Boundary & TCB** — 주소공간·예외·권한 중재와 사용자 공간 서비스 경계를 구체화.
2. **RFC-002 Object/Capability Model** — 생성·전달·종료·재사용·권한 회수의 상태 전이와 불변식.
3. **RFC-003/004/006 합동 검토** — IPC·메모리·Pager·부팅·서비스 재시작의 의존성 검증.
4. **RFC-009/010 초기 설계** — Native ABI와 Linux syscall 진입 메커니즘의 상충 여부 검토.
5. Reference VM/PC 요구사항, Rust/libc/Wine 의존성, 업데이트·라이선스 정책의 교차 검토.

## 14. 공식 커뮤니케이션 및 제한 사항

현재 사용할 수 있는 설명: **독립 데스크톱 운영체제 설계 프로젝트이며 Native 앱과 Linux/Windows/Darwin 생태계의 상호운용성을 연구 중**.

현재 사용할 수 없는 설명: **Linux/Windows/macOS 바이너리 실행 지원, 일반 macOS GUI/Metal 앱 지원, Apple의 승인·인증, 광범위한 실제 PC 지원, 일반 사용자용 보안 보장**. 검증되지 않은 기능은 연구 목표로만 표현한다.

macOS 호환성의 첫 검토 범위는 제한적인 x86_64 Mach-O/Darwin CLI이며 GUI·Swift/Objective-C 프레임워크, AppKit, Metal은 장기 고위험 분야다. Apple 독점 SDK/프레임워크 및 macOS OS 이미지를 재배포 가능한 구성요소로 가정하지 않는다.

이 문서는 설계 방향을 기록할 뿐, 특정 외부 OS나 앱의 호환성, 법적 사용 권리 또는 보안 인증을 약속하지 않는다.

## 15. 문서 이력 및 참조

- **v0.1:** Linux / Windows NT / Apple XNU / FreeBSD / Fuchsia Zircon 비교, 후보 아키텍처, 초기 RFC/ADR 제안. 보존.
- **v0.2:** 여덟 가지 상위 설계 결정의 방향 승인 기록, 아키텍처 경계, 우선 위험, ADR 색인 정규화, RFC·제품/보안 게이트, 연구 로드맵 통합. **코드 구현 및 실측 없음.**

참고 자료는 선행 [v0.1 문서의 원전 목록](kernel-architecture-and-feasibility-v0.1.md#13-출처--2026-10-24-확인한-1차-공식-문서프로젝트-문서) 및 아래 공식 문서에서 추적한다.

- [UEFI Specifications](https://uefi.org/specifications)
- [Rust x86_64-unknown-none platform support](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html)
- [seL4 Documentation](https://docs.sel4.systems/)
- [Fuchsia Architecture](https://fuchsia.dev/fuchsia-src/concepts)
- [Wine Source](https://github.com/wine-mirror/wine)
- [Darling Source](https://github.com/darlinghq/darling)
- [The Update Framework](https://theupdateframework.io/)
- [SPDX License List](https://spdx.org/licenses/)

**다음 승인 대상은 RFC 상세 설계와 이후 별도의 구현 Go/No-Go이며, 이 v0.2 공개만으로 구현을 시작하지 않는다.**
