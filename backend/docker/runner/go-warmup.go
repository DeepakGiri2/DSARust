// Building this file at image-build time compiles the standard-library
// packages the problem set uses into a shared Go build cache, so a job only
// ever compiles its own file (~0.3s) instead of the standard library (~10s).
//
// Every package listed here — and, transitively, everything it imports — ends
// up cached. It must stay a superset of what the harnesses and problems
// actually import; a missing entry only costs that job a one-time recompile,
// never a failure.
package main

import (
	"bufio"
	"bytes"
	"container/heap"
	"container/list"
	"errors"
	"fmt"
	"maps"
	"math"
	"os"
	"slices"
	"sort"
	"strconv"
	"strings"
	"unicode"
)

// An int-heap, so container/heap's generic machinery is instantiated and cached.
type intHeap []int

func (h intHeap) Len() int            { return len(h) }
func (h intHeap) Less(i, j int) bool  { return h[i] < h[j] }
func (h intHeap) Swap(i, j int)       { h[i], h[j] = h[j], h[i] }
func (h *intHeap) Push(x any)         { *h = append(*h, x.(int)) }
func (h *intHeap) Pop() any {
	old := *h
	n := len(old)
	x := old[n-1]
	*h = old[:n-1]
	return x
}

func main() {
	r := bufio.NewReader(os.Stdin)
	_ = r
	var buf bytes.Buffer
	fmt.Fprint(&buf, "warm ", strconv.Itoa(42), strings.ToUpper("cache"))

	h := &intHeap{2, 1, 5}
	heap.Init(h)
	heap.Push(h, 3)

	l := list.New()
	l.PushBack(1)

	nums := []int{3, 1, 2}
	sort.Ints(nums)
	slices.Sort(nums)
	_ = slices.Contains(nums, 2)
	m := map[string]int{"a": 1}
	_ = maps.Clone(m)

	_ = math.Sqrt(2)
	_ = unicode.IsLetter('a')
	_ = errors.New("warm")

	// Reference the accumulated work so nothing is optimised away.
	if buf.Len() < 0 {
		fmt.Println(nums, h, l.Len())
	}
}
