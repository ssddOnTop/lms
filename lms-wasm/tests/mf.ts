import {Miniflare, Response} from "miniflare"
import {MockAgent} from "undici"

const mockAgent = new MockAgent()

// mockAgent.get("fake.host").intercept({path: "/"}).reply(200, "Hello, world!", {headers: {"content-type": "text/plain"}})


export const mf = new Miniflare({
  scriptPath: "./build/worker/shim.mjs",
  cache: true,
  modules: true,
  modulesRules: [{type: "CompiledWasm", include: ["**/*.wasm"], fallthrough: true}],
  bindings: {BUCKET: "MY_R2"},
  r2Buckets: ["MY_R2"],
  fetchMock: mockAgent,
})
