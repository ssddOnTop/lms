import {describe, test, expect} from "vitest"
import {readFile} from "fs/promises"
import {mf} from "./mf"

describe("fetch", () => {
  test("loadfiles", async () => {
    let config = (await readFile("../examples/config.json")).toString()

    let bucket = await mf.getR2Bucket("MY_R2")
    await bucket.put("config.json", config)
  })

  test("hello_world", async () => {
    let resp = await mf.dispatchFetch("https://fake.host/helloworld?config=config.json", {
      method: "GET"
    })
    let body = await resp.text()
    let expected = "Hello World!"
    expect(body).toEqual(expected)
    expect(resp.status).toBe(200)
  })

  test("invalid_aes", async () => {
    let resp = await mf.dispatchFetch("https://fake.host/auth?config=config.json", {
      method: "POST",
      body: "invalid aes"
    })
    let body = await resp.json()
    let expected = {
      "error": {
        "message": "Unable to parse request"
      },
      "code": 500
    }
    expect(body).toEqual(expected)
    expect(resp.status).toBe(500)
  })

})
