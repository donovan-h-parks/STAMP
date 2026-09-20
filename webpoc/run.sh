#!/usr/bin/env bash
# One-command launch for the STAMP-web PoC: builds the React frontend and serves it
# (plus the API) from a single FastAPI/uvicorn process on http://localhost:8000
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
VENV="$ROOT/.venv313/bin"

"$VENV/python" -m pip install -q -r backend/requirements.txt
( cd frontend && npm install --no-audit --no-fund && npm run build )
cd backend
exec "$VENV/python" -m uvicorn app:app --host 127.0.0.1 --port 8000
