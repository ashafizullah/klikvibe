import { expect, test } from "bun:test";
import { branchUrl, commitUrl, fileUrl, newRequestUrl, parseRemote } from "./forge";

test("remote URLs become web addresses", () => {
  expect(parseRemote("git@github.com:ashafizullah/klikvibe.git")).toEqual({ forge: "github", base: "https://github.com/ashafizullah/klikvibe" });
  expect(parseRemote("https://github.com/ashafizullah/klikvibe")).toEqual({ forge: "github", base: "https://github.com/ashafizullah/klikvibe" });
  expect(parseRemote("https://oauth2:token@gitlab.com/group/sub/repo.git")).toEqual({ forge: "gitlab", base: "https://gitlab.com/group/sub/repo" });
  expect(parseRemote("ssh://git@gitlab.company.id:2222/team/app.git")).toEqual({ forge: "gitlab", base: "https://gitlab.company.id/team/app" });
  expect(parseRemote("git@bitbucket.org:team/app.git")?.forge).toBe("bitbucket");
  expect(parseRemote("git@git.example.com:me/x.git")).toEqual({ forge: "other", base: "https://git.example.com/me/x" });
  expect(parseRemote("/srv/git/repo.git")).toBeNull();
  expect(parseRemote("")).toBeNull();
});

test("links per forge", () => {
  const gh = parseRemote("git@github.com:o/r.git")!;
  const gl = parseRemote("git@gitlab.com:g/r.git")!;
  const bb = parseRemote("git@bitbucket.org:t/r.git")!;
  const other = parseRemote("git@git.example.com:o/r.git")!;
  expect(fileUrl(gh, "main", "src/app.ts", 12, 30)).toBe("https://github.com/o/r/blob/main/src/app.ts#L12-L30");
  expect(fileUrl(gl, "main", "src/app.ts", 12, 30)).toBe("https://gitlab.com/g/r/-/blob/main/src/app.ts#L12-30");
  expect(fileUrl(bb, "main", "src/app.ts", 12)).toBe("https://bitbucket.org/t/r/src/main/src/app.ts#lines-12");
  expect(fileUrl(gh, "feature/x", "a b.ts")).toBe("https://github.com/o/r/blob/feature/x/a%20b.ts");
  expect(commitUrl(gl, "abc123")).toBe("https://gitlab.com/g/r/-/commit/abc123");
  expect(branchUrl(gh, "feature/x")).toBe("https://github.com/o/r/tree/feature/x");
  expect(newRequestUrl(gh, "feature/x")).toBe("https://github.com/o/r/compare/feature/x?expand=1");
  expect(newRequestUrl(gl, "fix")).toBe("https://gitlab.com/g/r/-/merge_requests/new?merge_request%5Bsource_branch%5D=fix");
  expect(newRequestUrl(other, "fix")).toBeNull();
});
