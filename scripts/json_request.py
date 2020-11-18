import json
import requests

x = requests.get(
    'http://localhost:3030/convert/', 
    data=json.dumps({"words": ["hello", "world"]})
    )
print(x.status_code)