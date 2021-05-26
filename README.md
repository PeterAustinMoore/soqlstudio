# soqlstudio

## Setup
```bash
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
export SODA_USERNAME='apikey'
export SODA_PASSWORD='apitoken'
```

## Running
```bash
python app.py
```

## Building
```bash
cx_freeze app.py
```
