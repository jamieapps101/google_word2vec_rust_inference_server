import json,requests,time


def get_vector(words, url):
    if type(words) is str:
        return json.loads(requests.get(url,data=json.dumps({"words": [words]})).content)
    if type(words) is list and len(words)>0 and all([type(word) is str for word in words]):
        return json.loads(requests.get(url,data=json.dumps({"words": words})).content)


if __name__ == "__main__":
    # bench mark
    print("Mutli request")
    loops = 10000
    init_time = time.perf_counter()
    for i in range(loops):
        res = get_vector("hello", 'http://localhost:3030/convert/')
    finish_time = time.perf_counter()
    print("time per request: {} ms\n".format((finish_time-init_time)*1000/loops))

    print("Single request")
    entries = 10000
    init_time = time.perf_counter()
    res = get_vector(["hello" for i in range(entries)], 'http://localhost:3030/convert/')
    finish_time = time.perf_counter()
    print("time per entry: {} ms".format((finish_time-init_time)*1000/entries))



    