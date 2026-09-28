// Ported from the tests of `assemble` in crates/dsa-core/src/practice.rs.
import { describe, expect, it } from 'vitest'
import { assemble, locate, rustLines } from './assemble'

const GO =
  'func twoSum(nums []int, target int) []int {\n    seen := map[int]int{}\n    for i, x := range nums {\n        if j, ok := seen[target-x]; ok {\n            return []int{j, i}\n        }\n        seen[x] = i\n    }\n    return nil\n}\n'

const GO_HARNESS =
  'package main\n\nimport "fmt"\n\nfunc twoSum(nums []int, target int) []int {\n    seen := map[int]int{}\n    for i, x := range nums {\n        if j, ok := seen[target-x]; ok {\n            return []int{j, i}\n        }\n        seen[x] = i\n    }\n    return nil\n}\n\nfunc main() {\n    fmt.Println(twoSum([]int{2, 7}, 9))\n}\n'

/** What `dsa_core::practice::starter(GO, "go")` produces — the server's `ProblemSource.starter`. */
const GO_STARTER = 'func twoSum(nums []int, target int) []int {\n    // write your code here\n    panic("todo")\n}\n'

const count = (hay: string, needle: string) => hay.split(needle).length - 1

describe('assemble', () => {
  it("puts the user's code where the reference was", () => {
    const mine = 'func twoSum(nums []int, target int) []int {\n    return []int{0, 1}\n}\n'
    const program = assemble(mine, GO, GO_HARNESS)
    expect(program.startsWith('package main')).toBe(true)
    expect(program).toContain('return []int{0, 1}')
    expect(program).toContain('func main() {')
    // The reference must be gone, or the file has two twoSum functions.
    expect(program).not.toContain('seen := map[int]int{}')
    expect(count(program, 'func twoSum')).toBe(1)
  })

  it('re-indents the solution into a Java class', () => {
    const reference = 'static int f(int x) {\n    return x;\n}\n'
    const harness =
      'class Main {\n\n    static int f(int x) {\n        return x;\n    }\n\n    public static void main(String[] a) {}\n}\n'
    const mine = 'static int f(int x) {\n    return 42;\n}\n'
    const program = assemble(mine, reference, harness)
    expect(program).toContain('    static int f(int x) {\n        return 42;\n    }')
    expect(program).toContain('public static void main')
  })

  it('does not pad a blank line with the trailing indent', () => {
    const reference = 'static int f() {\n    return 1;\n}\n'
    const harness = 'class Main {\n    static int f() {\n        return 1;\n    }\n}\n'
    const mine = 'static int f() {\n\n    return 2;\n}\n'
    expect(assemble(mine, reference, harness)).toContain('{\n\n')
  })

  it('treats the solution as the program when there is no harness', () => {
    const mine = 'func twoSum() {}\n'
    expect(assemble(mine, GO, null)).toBe(mine)
    expect(assemble(mine, GO, '   ')).toBe(mine)
  })

  it('falls back rather than corrupting a harness the reference is missing from', () => {
    const mine = 'func twoSum() {}\n'
    expect(assemble(mine, GO, 'package main\nfunc main() {}\n')).toBe(mine)
  })

  it('round-trips the starter back through the harness', () => {
    // What the tab does on open: blank the reference, then run it. The result
    // has to still be a whole program.
    const program = assemble(GO_STARTER, GO, GO_HARNESS)
    expect(program).toContain('package main')
    expect(program).toContain('func main() {')
    expect(program).toContain('panic("todo")')
    expect(program).not.toContain('seen := map')
  })
})

describe('locate', () => {
  it('finds the reference by trimmed lines and reports the extra indent', () => {
    const harness = 'class Main {\n    static int f() {\n        return 1;\n    }\n}\n'
    expect(locate(harness, '\nstatic int f() {\n    return 1;\n}\n\n')).toEqual({ start: 1, end: 4, indent: '    ' })
  })

  it('refuses an empty reference or one longer than the harness', () => {
    expect(locate('a\nb\n', '  \n\n')).toBeNull()
    expect(locate('a\n', 'a\nb\nc\n')).toBeNull()
  })

  it('skips a candidate indented less than the reference', () => {
    expect(locate('x\ny\n', '    x\n    y\n')).toBeNull()
  })
})

describe('rustLines', () => {
  it("splits like Rust's str::lines", () => {
    expect(rustLines('')).toEqual([])
    expect(rustLines('a\n')).toEqual(['a'])
    expect(rustLines('a\n\nb')).toEqual(['a', '', 'b'])
    expect(rustLines('\n')).toEqual([''])
    expect(rustLines('a\r\nb\r\n')).toEqual(['a', 'b'])
  })
})
