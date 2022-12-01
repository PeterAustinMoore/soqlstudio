# soqlstudio

## Setup
```bash
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
export SODA_USERNAME='apikey'cx
export SODA_PASSWORD='apitoken'
```

## Running
```bash
python app.py
```

## Building
```bash
python setup.py bdist_mac
# for dev
codesign --remove-signature build/SoQLStudio-0.1.app/Contents/MacOS/lib/Python
```
