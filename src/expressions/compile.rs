//! AST -> bytecode.

extern crate alloc;

use alloc::vec::Vec;

use super::ast::{Cond, Node};
use super::opcode::{Inst, Program};
use super::parse::ParseResult;

pub fn compile(parsed: &ParseResult) -> Program {
    let mut insts = Vec::new();
    emit(&parsed.root, &mut insts);
    insts.push(Inst::Match);
    let mut history_groups = alloc::vec![false; parsed.capture_count];
    collect_history(&parsed.root, &mut history_groups);
    Program {
        insts,
        capture_count: parsed.capture_count,
        names: parsed.names.clone(),
        has_named: parsed.has_named,
        history_groups,
        lead: None,
        repeat_shapes: Vec::new(),
        ascii_literal: None,
        group_spans: Vec::new(),
        class_plans: Vec::new(),
        literal_bytes: Vec::new(),
        req_lit: None,
        anchored_bol: false,
    }
}

fn collect_history(node: &Node, hist: &mut Vec<bool>) {
    match node {
        Node::Capture {
            index,
            inner,
            history,
            ..
        } => {
            if *history {
                if let Some(slot) = hist.get_mut(*index) {
                    *slot = true;
                }
            }
            collect_history(inner, hist);
        }
        Node::Concat(v) | Node::Alt(v) => {
            for n in v {
                collect_history(n, hist);
            }
        }
        Node::Repeat { inner, .. }
        | Node::Group(inner)
        | Node::Look { inner, .. }
        | Node::Atomic(inner)
        | Node::Options { inner, .. } => collect_history(inner, hist),
        Node::Absent { stopper, expr, .. } => {
            collect_history(stopper, hist);
            if let Some(e) = expr {
                collect_history(e, hist);
            }
        }
        Node::Conditional {
            then_n, else_n, ..
        } => {
            collect_history(then_n, hist);
            if let Some(e) = else_n {
                collect_history(e, hist);
            }
        }
        _ => {}
    }
}

fn flush_chars(buf: &mut Vec<u32>, out: &mut Vec<Inst>) {
    match buf.as_slice() {
        [] => {}
        [c] => out.push(Inst::Char(*c)),
        _ => out.push(Inst::Literal(core::mem::take(buf))),
    }
    buf.clear();
}

fn emit(node: &Node, out: &mut Vec<Inst>) {
    match node {
        Node::Empty => {}
        Node::Char(c) => out.push(Inst::Char(*c)),
        Node::Literal(v) => out.push(Inst::Literal(v.clone())),
        Node::Any { newline } => out.push(Inst::Any { newline: *newline }),
        Node::SuperAny => out.push(Inst::SuperAny),
        Node::Class(cc) => out.push(Inst::Class { class: cc.clone() }),
        Node::Concat(v) => {
            let mut buf = Vec::new();
            for n in v {
                if let Node::Char(c) = n {
                    buf.push(*c);
                } else {
                    flush_chars(&mut buf, out);
                    emit(n, out);
                }
            }
            flush_chars(&mut buf, out);
        }
        Node::Alt(v) => emit_alt(v, out),
        Node::Repeat {
            inner,
            min,
            max,
            greedy,
            possessive,
        } => {
            let rec = out.len();
            out.push(Inst::Nop);
            let body = out.len() as u16;
            emit(inner, out);
            let after = (out.len() + 1) as u16;
            out.push(Inst::Jump(0));
            let after_real = out.len() as u16;
            out[rec] = Inst::Repeat {
                body,
                after: after_real,
                min: *min,
                max: *max,
                greedy: *greedy,
                possessive: *possessive,
            };
            if let Inst::Jump(j) = &mut out[after as usize - 1] {
                *j = after_real;
            }
            let _ = after;
        }
        Node::Capture { index, inner, .. } => {
            let slot = (*index as u16) * 2;
            out.push(Inst::Save(slot));
            emit(inner, out);
            out.push(Inst::Save(slot + 1));
        }
        Node::Group(inner) | Node::Options { inner, .. } => {
            if let Node::Options { set, clear, inner } = node {
                out.push(Inst::PushOptions(*set, *clear));
                emit(inner, out);
                out.push(Inst::PopOptions);
            } else {
                emit(inner, out);
            }
        }
        Node::Anchor(a) => out.push(Inst::Assert(*a)),
        Node::Backref(b) => out.push(Inst::Backref(b.clone())),
        Node::Look {
            behind,
            negative,
            inner,
        } => {
            let rec = out.len();
            out.push(Inst::Nop);
            let body = out.len() as u16;
            emit(inner, out);
            let after = out.len() as u16 + 1;
            out.push(Inst::Jump(after));
            let after_real = out.len() as u16;
            out[rec] = Inst::Look {
                body,
                after: after_real,
                behind: *behind,
                negative: *negative,
            };
        }
        Node::Atomic(inner) => {
            let rec = out.len();
            out.push(Inst::Nop);
            let body = out.len() as u16;
            emit(inner, out);
            out.push(Inst::Jump(0));
            let after_real = out.len() as u16;
            out[rec] = Inst::Atomic {
                body,
                after: after_real,
            };
            let jump_at = out.len() - 1;
            if let Inst::Jump(j) = &mut out[jump_at] {
                *j = after_real;
            }
        }
        Node::Call(t) => out.push(Inst::Call(t.clone())),
        Node::Absent {
            stopper,
            expr,
            kind,
        } => {
            let rec = out.len();
            out.push(Inst::Nop);
            let st = out.len() as u16;
            emit(stopper, out);
            let exp = if let Some(e) = expr {
                let p = out.len() as u16;
                emit(e, out);
                Some(p)
            } else {
                None
            };
            let after = out.len() as u16;
            out[rec] = Inst::Absent {
                stopper: st,
                expr: exp,
                after,
                kind: *kind,
            };
        }
        Node::Conditional {
            cond,
            then_n,
            else_n,
        } => {
            let cond_look = if let Cond::Expr(e) = cond {
                let skip_at = out.len();
                out.push(Inst::Jump(0));
                let body = out.len() as u16;
                emit(e, out);
                let after_c = out.len() as u16;
                if let Inst::Jump(j) = &mut out[skip_at] {
                    *j = after_c;
                }
                Some((body, after_c))
            } else {
                None
            };
            let rec = out.len();
            out.push(Inst::Nop);
            let then_pc = out.len() as u16;
            emit(then_n, out);
            out.push(Inst::Jump(0));
            let else_pc = else_n.as_ref().map(|e| {
                let p = out.len() as u16;
                emit(e, out);
                p
            });
            let after = out.len() as u16;
            for inst in out.iter_mut().rev() {
                if let Inst::Jump(j) = inst {
                    if *j == 0 {
                        *j = after;
                        break;
                    }
                }
            }
            let cond = if let Some((body, after_c)) = cond_look {
                Cond::Look { body, after: after_c }
            } else {
                cond.clone()
            };
            out[rec] = Inst::Cond {
                cond,
                then_pc,
                else_pc,
                after,
            };
        }
        Node::Keep => out.push(Inst::Keep),
        Node::GeneralNewline => out.push(Inst::GeneralNewline),
        Node::TextSegment => out.push(Inst::TextSegment),
        Node::Callout {
            named,
            name,
            args,
            tag,
            body,
            dir,
        } => out.push(Inst::Callout {
            named: *named,
            name: name.clone(),
            args: args.clone(),
            tag: tag.clone(),
            body: body.clone(),
            dir: *dir,
        }),
    }
}

fn emit_alt(v: &[Node], out: &mut Vec<Inst>) {
    if v.is_empty() {
        return;
    }
    if v.len() == 1 {
        emit(&v[0], out);
        return;
    }
    let mut holes: Vec<usize> = Vec::new();
    for (i, n) in v.iter().enumerate() {
        if i + 1 < v.len() {
            let split_at = out.len();
            out.push(Inst::Split(0, 0));
            let l1 = out.len() as u16;
            emit(n, out);
            holes.push(out.len());
            out.push(Inst::Jump(0));
            let l2 = out.len() as u16;
            if let Inst::Split(a, b) = &mut out[split_at] {
                *a = l1;
                *b = l2;
            }
        } else {
            emit(n, out);
        }
    }
    let end = out.len() as u16;
    for h in holes {
        if let Inst::Jump(j) = &mut out[h] {
            *j = end;
        }
    }
}
