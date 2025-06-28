
# SSH Connector (목적 및 프로젝트 개요)

사용자의 `~/.ssh/config` 파일에 있는 호스트 목록을 대화형 메뉴로 제공하여 SSH 연결을 간소화하는 CLI 도구입니다. 사용자가 SSH 호스트를 쉽게 필터링, 선택 및 연결할 수 있도록 지원합니다.

## Rules

- **커밋:** Conventional Commits 사양을 따릅니다.
- **버전 관리:** 시맨틱 버전 관리(vX.X.X)를 사용합니다.
- **포맷팅:** 코드는 `black`과 `isort`로 포맷팅합니다.
- **코드 품질:** `black`, `isort`, `pytest`를 모두 통과한 상태에서만 GitHub에 커밋 및 푸시합니다.

## Requirement

- **언어:** Python 3.13+
- **패키지 관리자:** `uv`

## Environment

- **테스팅:** `pytest`
- **의존성:** 외부 의존성의 수를 최소한으로 유지하고, 가능하면 표준 라이브러리를 사용합니다.
