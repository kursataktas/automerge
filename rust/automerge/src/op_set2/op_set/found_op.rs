use crate::types::Clock;

use super::{Action, KeyRef, Op, OpQueryTerm, OpsFound};

use std::fmt::Debug;

#[derive(Clone, Debug)]
pub(crate) struct OpsFoundIter<'a, I: Iterator<Item = Op<'a>>> {
    iter: I,
    clock: Option<Clock>,
    last_key: Option<KeyRef<'a>>,
    found: Option<OpsFound<'a>>,
}

impl<'a, I: Iterator<Item = Op<'a>>> OpsFoundIter<'a, I> {
    pub(crate) fn new(iter: I, clock: Option<Clock>) -> Self {
        Self {
            iter,
            clock,
            found: None,
            last_key: None,
        }
    }
}

impl<'a, I: OpQueryTerm<'a>> Iterator for OpsFoundIter<'a, I> {
    type Item = OpsFound<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        while let Some(mut op) = self.iter.next() {
            if op.action == Action::Increment {
                continue;
            }
            let key = op.elemid_or_key();
            if Some(key) != self.last_key {
                result = self.found.take();
                self.last_key = Some(key);
                self.found = Some(OpsFound::default());
            }
            if let Some(found) = &mut self.found {
                found.end_pos = op.pos + 1;
                if op.scope_to_clock(self.clock.as_ref(), self.iter.get_opiter()) {
                    // FIXME we dont need this b/c of ops.index
                    found.ops_pos.push(op.pos);
                    found.ops.push(op);
                }
            }
            match &result {
                Some(f) if !f.ops.is_empty() => return result,
                _ => (),
            }
        }
        self.found.take()
    }
}