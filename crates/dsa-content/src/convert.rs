//! Conversions between Rhai's dynamic values and the trace model, plus the
//! compact encodings authors use for trees, grids and linked lists.
//!
//! Every converter is forgiving about shape (an index list may be `[1,2]` or a
//! bare `1`; map entries may be a Rhai map or a list of pairs) and precise
//! about errors — a content author reads these messages, so they name the
//! offending value rather than a Rust type.

use dsa_core::model::{Cell, ListNode, TreeNodeV, VarVal};
use rhai::{Dynamic, EvalAltResult, Map, Position, FLOAT, INT};

pub type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

pub fn err<T>(msg: impl Into<String>) -> RhaiResult<T> {
    Err(Box::new(EvalAltResult::ErrorRuntime(
        msg.into().into(),
        Position::NONE,
    )))
}

fn type_of(d: &Dynamic) -> String {
    d.type_name()
        .rsplit("::")
        .next()
        .unwrap_or("value")
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Scalars
// ─────────────────────────────────────────────────────────────────────────────

pub fn to_cell(d: &Dynamic) -> RhaiResult<Cell> {
    if d.is_unit() {
        return Ok(Cell::Str(String::new()));
    }
    if let Some(i) = d.clone().try_cast::<INT>() {
        return Ok(Cell::Num(i as f64));
    }
    if let Some(f) = d.clone().try_cast::<FLOAT>() {
        return Ok(Cell::Num(f));
    }
    if let Some(b) = d.clone().try_cast::<bool>() {
        return Ok(Cell::Str(b.to_string()));
    }
    if let Some(c) = d.clone().try_cast::<char>() {
        return Ok(Cell::Str(c.to_string()));
    }
    if d.is_string() {
        return Ok(Cell::Str(d.clone().into_string().unwrap_or_default()));
    }
    err(format!("expected a number or string, got {}", type_of(d)))
}

/// Accepts an array, a string (each character becomes a cell) or a single
/// scalar. Strings-as-arrays is what makes `array("s", s)` work for the string
/// problems without the author splitting first.
pub fn to_cells(d: &Dynamic) -> RhaiResult<Vec<Cell>> {
    if d.is_array() {
        let arr = d.clone().into_array().map_err(rt)?;
        return arr.iter().map(to_cell).collect();
    }
    if d.is_string() {
        let s = d.clone().into_string().unwrap_or_default();
        return Ok(s.chars().map(|c| Cell::Str(c.to_string())).collect());
    }
    if d.is_unit() {
        return Ok(vec![]);
    }
    Ok(vec![to_cell(d)?])
}

fn rt(e: &str) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        e.to_string().into(),
        Position::NONE,
    ))
}

pub fn to_i64(d: &Dynamic) -> RhaiResult<i64> {
    if let Some(i) = d.clone().try_cast::<INT>() {
        return Ok(i);
    }
    if let Some(f) = d.clone().try_cast::<FLOAT>() {
        return Ok(f as i64);
    }
    if d.is_string() {
        if let Ok(v) = d
            .clone()
            .into_string()
            .unwrap_or_default()
            .trim()
            .parse::<i64>()
        {
            return Ok(v);
        }
    }
    err(format!("expected an integer, got {}", type_of(d)))
}

/// `[1, 2]`, `1`, or `()` — all mean a list of indices.
pub fn to_i64_list(d: &Dynamic) -> RhaiResult<Vec<i64>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    if d.is_array() {
        let arr = d.clone().into_array().map_err(rt)?;
        return arr.iter().map(to_i64).collect();
    }
    Ok(vec![to_i64(d)?])
}

pub fn to_opt_i64(d: &Dynamic) -> RhaiResult<Option<i64>> {
    if d.is_unit() {
        return Ok(None);
    }
    Ok(Some(to_i64(d)?))
}

pub fn to_string_list(d: &Dynamic) -> RhaiResult<Vec<String>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    if d.is_array() {
        let arr = d.clone().into_array().map_err(rt)?;
        return arr
            .iter()
            .map(|x| to_cell(x).map(|c| c.to_string()))
            .collect::<RhaiResult<_>>();
    }
    Ok(vec![to_cell(d)?.to_string()])
}

/// `[[r, c], ...]` — also accepts a single `[r, c]` pair.
pub fn to_rc_list(d: &Dynamic) -> RhaiResult<Vec<(i64, i64)>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    let arr = d
        .clone()
        .into_array()
        .map_err(|_| rt("expected a list of [row, col] pairs"))?;
    if arr.is_empty() {
        // "no cells" is the overwhelmingly common case at the start of a
        // trace; it must not be mistaken for a malformed pair.
        return Ok(vec![]);
    }
    if arr.iter().all(|x| !x.is_array()) {
        // A bare [r, c].
        if arr.len() == 2 {
            return Ok(vec![(to_i64(&arr[0])?, to_i64(&arr[1])?)]);
        }
        return err("expected [row, col] pairs");
    }
    arr.iter()
        .map(|p| {
            let pair = p
                .clone()
                .into_array()
                .map_err(|_| rt("expected [row, col]"))?;
            if pair.len() != 2 {
                return err("each cell reference needs exactly [row, col]");
            }
            Ok((to_i64(&pair[0])?, to_i64(&pair[1])?))
        })
        .collect()
}

pub fn to_rc(d: &Dynamic) -> RhaiResult<Option<(i64, i64)>> {
    if d.is_unit() {
        return Ok(None);
    }
    let pair = d
        .clone()
        .into_array()
        .map_err(|_| rt("expected [row, col]"))?;
    if pair.len() != 2 {
        return err("expected [row, col]");
    }
    Ok(Some((to_i64(&pair[0])?, to_i64(&pair[1])?)))
}

/// `#{ i: 0, j: 3 }` or `[["i", 0], ["j", 3]]`.
pub fn to_pointers(d: &Dynamic) -> RhaiResult<Vec<(String, i64)>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    if let Some(map) = d.clone().try_cast::<Map>() {
        return map
            .iter()
            .map(|(k, v)| Ok((k.to_string(), to_i64(v)?)))
            .collect();
    }
    let arr = d
        .clone()
        .into_array()
        .map_err(|_| rt("expected #{ name: index } pointers"))?;
    arr.iter()
        .map(|p| {
            let pair = p
                .clone()
                .into_array()
                .map_err(|_| rt("expected [name, index]"))?;
            if pair.len() != 2 {
                return err("expected [name, index]");
            }
            Ok((to_cell(&pair[0])?.to_string(), to_i64(&pair[1])?))
        })
        .collect()
}

/// Like [`to_pointers`] but a unit value means "nil pointer", which linked
/// lists need in order to draw `next -> nil`.
pub fn to_opt_pointers(d: &Dynamic) -> RhaiResult<Vec<(String, Option<i64>)>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    if let Some(map) = d.clone().try_cast::<Map>() {
        return map
            .iter()
            .map(|(k, v)| Ok((k.to_string(), to_opt_i64(v)?)))
            .collect();
    }
    let arr = d
        .clone()
        .into_array()
        .map_err(|_| rt("expected #{ name: id } pointers"))?;
    arr.iter()
        .map(|p| {
            let pair = p
                .clone()
                .into_array()
                .map_err(|_| rt("expected [name, id]"))?;
            Ok((to_cell(&pair[0])?.to_string(), to_opt_i64(&pair[1])?))
        })
        .collect()
}

/// Map entries: `#{ a: 1 }`, `[["a", 1], ...]`, or an empty unit.
/// Rhai maps sort their keys, so the pair-list form is what preserves
/// insertion order — which is how a hash map animation reads correctly.
pub fn to_entries(d: &Dynamic) -> RhaiResult<Vec<(String, Cell)>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    if let Some(map) = d.clone().try_cast::<Map>() {
        return map
            .iter()
            .map(|(k, v)| Ok((k.to_string(), to_cell(v)?)))
            .collect();
    }
    let arr = d
        .clone()
        .into_array()
        .map_err(|_| rt("expected map entries as #{k: v} or [[k, v], ...]"))?;
    arr.iter()
        .map(|p| {
            let pair = p
                .clone()
                .into_array()
                .map_err(|_| rt("expected [key, value]"))?;
            if pair.len() != 2 {
                return err("each map entry needs exactly [key, value]");
            }
            Ok((to_cell(&pair[0])?.to_string(), to_cell(&pair[1])?))
        })
        .collect()
}

pub fn to_var(d: &Dynamic) -> RhaiResult<VarVal> {
    if let Some(v) = d.clone().try_cast::<VarVal>() {
        return Ok(v);
    }
    // Bare values are accepted so `set(#{ i: 3 })` works without wrapping.
    if d.is_unit() {
        return Ok(VarVal::Null);
    }
    if let Some(b) = d.clone().try_cast::<bool>() {
        return Ok(VarVal::Bool { v: b });
    }
    if d.is_string() {
        return Ok(VarVal::Str {
            v: d.clone().into_string().unwrap_or_default(),
        });
    }
    if d.is_array() {
        return Ok(VarVal::Arr {
            v: to_cells(d)?,
            hl: vec![],
        });
    }
    if let Some(map) = d.clone().try_cast::<Map>() {
        let entries = map
            .iter()
            .map(|(k, v)| Ok((k.to_string(), to_cell(v)?)))
            .collect::<RhaiResult<_>>()?;
        return Ok(VarVal::Map {
            v: entries,
            hl: vec![],
        });
    }
    Ok(VarVal::Num { v: to_cell(d)? })
}

pub fn to_vars(d: &Dynamic) -> RhaiResult<Vec<(String, VarVal)>> {
    if d.is_unit() {
        return Ok(vec![]);
    }
    let map = d
        .clone()
        .try_cast::<Map>()
        .ok_or_else(|| rt("expected variables as #{ name: value }"))?;
    map.iter()
        .map(|(k, v)| Ok((k.to_string(), to_var(v)?)))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Compact structure encodings
// ─────────────────────────────────────────────────────────────────────────────

/// Level-order tree text: `"4 2 7 1 3 n 9"`, where `n`, `null`, `#` and `.`
/// all mean "no child". Ids are assigned in creation order, so a script can
/// refer to a node by its position in the input.
///
/// Returns `(nodes, root_id)`.
pub fn parse_tree(spec: &str) -> (Vec<TreeNodeV>, Option<i64>) {
    let toks: Vec<&str> = spec
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .collect();
    let is_nil = |t: &str| matches!(t, "n" | "N" | "null" | "nil" | "#" | "." | "-");

    let mut nodes: Vec<TreeNodeV> = Vec::new();
    if toks.is_empty() || is_nil(toks[0]) {
        return (nodes, None);
    }

    // BFS assignment: the classic LeetCode level-order layout where nil
    // children do not consume slots for their own descendants.
    let mut queue: Vec<usize> = Vec::new();
    nodes.push(TreeNodeV {
        id: 0,
        val: cell_of(toks[0]),
        left: None,
        right: None,
    });
    queue.push(0);

    let mut qi = 0;
    let mut ti = 1;
    while qi < queue.len() && ti < toks.len() {
        let parent = queue[qi];
        qi += 1;
        for side in 0..2 {
            if ti >= toks.len() {
                break;
            }
            let tok = toks[ti];
            ti += 1;
            if is_nil(tok) {
                continue;
            }
            let id = nodes.len() as i64;
            nodes.push(TreeNodeV {
                id,
                val: cell_of(tok),
                left: None,
                right: None,
            });
            if side == 0 {
                nodes[parent].left = Some(id);
            } else {
                nodes[parent].right = Some(id);
            }
            queue.push(id as usize);
        }
    }
    (nodes, Some(0))
}

fn cell_of(tok: &str) -> Cell {
    match tok.parse::<f64>() {
        Ok(n) => Cell::Num(n),
        Err(_) => Cell::Str(tok.to_string()),
    }
}

/// Grid text: rows separated by whitespace, one character per cell
/// (`"11000 11000"`). Also accepts an array of strings or an array of arrays.
pub fn parse_grid(d: &Dynamic) -> RhaiResult<Vec<Vec<Cell>>> {
    if d.is_string() {
        let s = d.clone().into_string().unwrap_or_default();
        return Ok(s
            .split_whitespace()
            .map(|row| row.chars().map(|c| Cell::Str(c.to_string())).collect())
            .collect());
    }
    let arr = d
        .clone()
        .into_array()
        .map_err(|_| rt("expected grid rows"))?;
    arr.iter()
        .map(|row| {
            if row.is_string() {
                let s = row.clone().into_string().unwrap_or_default();
                // A row of digits is split per character; a row of space
                // separated numbers is split on whitespace.
                if s.contains(' ') {
                    Ok(s.split_whitespace().map(cell_of).collect())
                } else {
                    Ok(s.chars().map(|c| Cell::Str(c.to_string())).collect())
                }
            } else {
                to_cells(row)
            }
        })
        .collect()
}

/// Build a linked list from plain values: ids are `0..n`, `next` chains them.
pub fn list_from_values(values: &[Cell]) -> Vec<ListNode> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| ListNode {
            id: i as i64,
            val: v.clone(),
            next: if i + 1 < values.len() {
                Some(i as i64 + 1)
            } else {
                None
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_order_skips_nil_children_correctly() {
        let (nodes, root) = parse_tree("4 2 7 1 3 n 9");
        assert_eq!(root, Some(0));
        assert_eq!(nodes.len(), 6);
        assert_eq!(nodes[0].val.to_string(), "4");
        // 2 -> children 1, 3
        assert_eq!(
            nodes[1].left.map(|i| nodes[i as usize].val.to_string()),
            Some("1".into())
        );
        assert_eq!(
            nodes[1].right.map(|i| nodes[i as usize].val.to_string()),
            Some("3".into())
        );
        // 7 -> nil left, 9 right
        assert_eq!(nodes[2].left, None);
        assert_eq!(
            nodes[2].right.map(|i| nodes[i as usize].val.to_string()),
            Some("9".into())
        );
    }

    #[test]
    fn empty_and_nil_only_trees_are_empty() {
        assert_eq!(parse_tree("").1, None);
        assert_eq!(parse_tree("n").1, None);
        assert!(parse_tree("  ").0.is_empty());
    }

    #[test]
    fn tree_accepts_negative_and_string_values() {
        let (nodes, _) = parse_tree("-3 a");
        assert_eq!(nodes[0].val.to_string(), "-3");
        assert_eq!(nodes[1].val.to_string(), "a");
    }

    #[test]
    fn grid_text_becomes_one_cell_per_character() {
        let g = parse_grid(&Dynamic::from("110 011")).unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].len(), 3);
        assert_eq!(g[0][0].to_string(), "1");
    }

    #[test]
    fn an_empty_cell_list_is_not_a_malformed_pair() {
        let empty: rhai::Array = vec![];
        assert_eq!(to_rc_list(&Dynamic::from(empty)).unwrap(), vec![]);
        assert_eq!(to_rc_list(&Dynamic::UNIT).unwrap(), vec![]);
    }

    #[test]
    fn linked_list_ids_chain_in_order() {
        let l = list_from_values(&[1.into(), 2.into(), 3.into()]);
        assert_eq!(l[0].next, Some(1));
        assert_eq!(l[2].next, None);
        assert_eq!(l.len(), 3);
    }

    #[test]
    fn strings_convert_to_one_cell_per_character() {
        let cells = to_cells(&Dynamic::from("abc")).unwrap();
        assert_eq!(cells.len(), 3);
        assert_eq!(cells[1].to_string(), "b");
    }
}
