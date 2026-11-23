[English](README.md) | **한국어** | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

# bmo-theater

플랫폼의 동영상 저장소를 하나의 서비스로 제공합니다. 에피소드로 이루어진 타이틀을 다루며, 그 출처는 출력물로서 자체
파일 루트에 등록된 동영상과, 이미 디스크에 있는 동영상 파일 라이브러리(제자리에서 색인)입니다. 각 에피소드는 HTTP
범위 요청으로 파일 그대로 스트리밍하고, 자막은 WebVTT로 변환하며 요청하면 영상에 입혀(burn-in) 줍니다. 재생 목록,
시청자별 시청 위치와 다음에 볼 항목, 로그인 없이 동영상을 여는 공유 링크, 회사의 특정 계정에 동영상을 열어 주는
권한 부여를 제공합니다. 이 모든 것은 서로 분리된 두 영역, 즉 각 계정 자신의 영역과 조직의 영역에 나뉘어 보관됩니다.
또한 콘솔 사이드바 항목 아래에서 이를 보여 주는 콘솔 번들도 포함합니다. `docs/design.md`는 이 서비스가 무엇인지를,
`docs/api.md`는 그 경계, 즉 모든 라우트, 발행하는 이벤트, 배치하는 도구, 실행하는 명령, 설정을 설명합니다.

## 테스트

`cargo test`(testbed를 통해 실행)는 크레이트의 유일한 하네스인 `tests/main.rs`를 실행합니다. 각 테스트는 스위트 서버에
있는 자기 전용 PostgreSQL 데이터베이스와 자기 전용 파일 루트 위에서 돌아갑니다. 다루는 내용은 다음과 같습니다. 파일
이름에서 번호 읽기(`names.rs`), 자막 형식 변환(`subtitles.rs`), 범위 응답(`range.rs`), 진행 상황과 다음
에피소드(`progress.rs`), ffmpeg로 만든 아주 작은 동영상으로 라이브러리를 계획하고 스캔하고 스트리밍하고 다시 스캔하고
자막을 입히는 과정과 업로드 및 경로로 동영상을 등록하는 과정(`scan.rs`, ffmpeg가 설치되지 않은 곳에서는 건너뜀),
영역과 접근 가드, 재생 목록과 위치(`realms.rs`), 타이틀 목록의 정렬 순서(`orders.rs`), 이어 보기와 다음
에피소드(`watching.rs`), 작품의 이름 변경·이동·삭제(`work.rs`), 공유 링크(`shares.rs`), 권한 부여(`grants.rs`), 콘솔
화면(`console.rs`), OpenAPI 문서(`openapi.rs`), 저장소 자체 검사(`repository.rs`). 도구의 크레이트는 대역 theater
서비스를 상대로 하는 자체 하네스 `tools/theater/tests/main.rs`를 가집니다. 콘솔 번들의 테스트는 `client/tests/`에 있으며,
Bun으로 만든 대역 서비스를 상대로 `bun run check`가 실행합니다.

## 구성

| 폴더 | 내용 |
|---|---|
| `src/` | 서비스와 그 명령: 라우트, 테이블과 마이그레이션, 스캔, 자막 입히기, 자막 변환기, 범위 응답, 콘솔 화면 연결, OpenAPI 문서 |
| `tools/` | 배포 시 도구 호스트에 배치되는 theater 도구: 크레이트, 매니페스트, 입력 스키마, 매뉴얼 |
| `client/` | 콘솔 플러그인 번들. 독립된 프로젝트입니다 |
| `deploy/` | 서비스의 사용자 유닛 |
| `docs/` | 서비스가 무엇인지와 그 경계를 정한 정본 문서, 그리고 OpenAPI 문서 |
| `tests/` | 크레이트의 유일한 테스트 하네스와 모듈들이 공유하는 코드 |

루트에는 Cargo 매니페스트, `Cargo.lock`, `testbed.toml`, `rustfmt.toml`, `settings.toml`(바이너리에 내장되는 기본값),
`CLAUDE.md`, `PERSONAL-DATA.md`, `LICENSE`, 그리고 이 README와 그 한국어·일본어·중국어 번역이 있습니다.

## 명령

- `bmo-theater serve`는 데이터베이스를 현재 스키마로 맞춘 뒤, 중지 요청을 받을 때까지 라우트에 응답합니다.
- `bmo-theater migrate`는 데이터베이스를 현재 스키마로 맞추고 종료합니다.
- `bmo-theater health`는 설정에 지정된 서비스가 준비 상태로 응답하면 0으로 종료합니다.
- `bmo-theater openapi`는 `docs/openapi.json`에 담긴 OpenAPI 문서를 출력합니다.
- `bmo-theater console-reach`는 콘솔이 서비스에 닿는 방법을 하나의 JSON 객체로 출력합니다. 배포의 콘솔 점검에
  쓰입니다.
- `bmo-theater tool-env`는 도구가 서비스를 찾는 데 쓰는 데이터 루트의 `env/theater.env`를 작성합니다.

설정은 바이너리에 내장된 `settings.toml`, 그다음 릴리스 옆에 있는 배포의 `settings.toml`과 `settings.local.toml`(데이터
루트의 `env/theater.toml`), 마지막으로 환경 변수 `BMO_THEATER_<TABLE>__<KEY>` 순서로 읽습니다(`docs/api.md`,
「Settings」).
