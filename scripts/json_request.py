import json
import requests

x = requests.get('http://localhost:3030/convert/', data=json.dumps({"data": "hello"}))
print(x.status_code)