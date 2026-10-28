import { test } from "node:test";
import assert from "node:assert/strict";
import { desembrulhar } from "../../renderer/js/api.js";

test("devolve o value quando ok", async () => {
  assert.equal(await desembrulhar(Promise.resolve({ ok: true, value: 1 })), 1);
});

test("rejeita com Error que carrega o code e a message", async () => {
  await assert.rejects(
    desembrulhar(Promise.resolve({ ok: false, code: "DB_INVALID", message: "x" })),
    (e) => e instanceof Error && e.code === "DB_INVALID" && e.message === "x",
  );
});
