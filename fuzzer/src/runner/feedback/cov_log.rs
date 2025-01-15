use std::{
    borrow::Cow,
    fs::{self, OpenOptions},
    io::Write,
    marker::PhantomData,
};

use libafl::{
    feedbacks::{Feedback, StateInitializer},
    observers::MapObserver,
    state::HasExecutions,
    HasNamedMetadata, SerdeAny,
};
use libafl_bolts::{
    tuples::{Handle, MatchNameRef},
    AsSlice, Named,
};
use serde::{Deserialize, Serialize};

pub struct CovLogFeedback<H, O> {
    handle: H,
    path: String,
    phantom: PhantomData<O>,
}

impl<H, O> CovLogFeedback<H, O> {
    pub fn new(handle: H, id: usize) -> Self {
        fs::create_dir_all("./cov-log-feedback/").unwrap();

        Self {
            handle,
            path: format!("./cov-log-feedback/{}.txt", id),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, SerdeAny, Serialize, Deserialize)]
struct CovLogFeedbackMetadata {
    map: Vec<(Vec<u8>, usize)>,
}

impl<OO, O, EM, I, OT, S> Feedback<EM, I, OT, S> for CovLogFeedback<Handle<OO>, O>
where
    OT: MatchNameRef,
    OO: AsRef<O>,
    O: MapObserver + for<'a> AsSlice<'a, Entry = u8>,
    S: HasNamedMetadata + HasExecutions,
{
    fn is_interesting(
        &mut self,
        state: &mut S,
        _manager: &mut EM,
        _input: &I,
        observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        let executions = *state.executions();
        let log = &mut state
            .named_metadata_or_insert_with("cov-log-feedback", || CovLogFeedbackMetadata {
                map: Vec::new(),
            })
            .map;

        let o = observers.get(&self.handle).unwrap().as_ref();
        let current = o.as_slice();
        match log.iter_mut().find(|(e, _c)| {
            e.iter()
                .zip(current.iter())
                .all(|(a, b)| (*a == 0) == (*b == 0))
        }) {
            Some((_e, c)) => *c += 1,
            None => {
                log.push((current.to_vec(), 1));
            }
        }

        if executions % 10 == 0 {
            log.sort_by_key(|e| e.1);
            log.reverse();
            // println!(
            //     "=== {}: {:?}",
            //     log.len(),
            //     log.iter().map(|e| e.1).collect::<Vec<_>>()
            // );
            let (std, cmp) = log.split_at(1);
            let std = &std[0];
            let mut diff_counts = vec![0; std.0.len()];
            for c in cmp {
                std.0
                    .iter()
                    .zip(c.0.iter())
                    .enumerate()
                    .filter(|(_, (e1, e2))| **e1 != **e2)
                    .for_each(|(i, _)| diff_counts[i] += c.1);
            }
            // let mut offsets = log
            //     .iter()
            //     .enumerate()
            //     .flat_map(|(i1, (v1, _))| {
            //         log.iter()
            //             .skip(i1 + 1)
            //             .flat_map(|(v2, _)| {
            //                 v1.iter()
            //                     .zip(v2.iter())
            //                     .enumerate()
            //                     .filter(|(_offset, (e1, e2))| **e1 != **e2)
            //                     .map(|(offset, _)| offset)
            //                     .collect::<Vec<_>>()
            //             })
            //             .collect::<Vec<_>>()
            //     })
            //     .collect::<Vec<_>>();

            // offsets.sort();
            // offsets.dedup();

            // let s = log.iter().fold(
            //     format!("After {} executions\n", executions),
            //     |acc, (k, v)| format!("{acc}{v: >10}: {}\n", hex::encode(k)),
            // );
            // let s = String::new();

            // let s = offsets
            //     .iter()
            //     .fold(s, |acc, e| format!("{acc}case {}:\n", e / 4));
            let s = diff_counts
                .iter()
                .enumerate()
                .filter(|(_, c)| **c > 0)
                .fold(format!("compare: {}\n", std.1), |acc, (i, c)| {
                    format!("{acc}case {i}: //{c}\n")
                });

            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.path)
                .unwrap()
                .write_all(s.as_bytes())
                .unwrap();
        }

        Ok(false)
    }
}

impl<H, O> Named for CovLogFeedback<H, O> {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("CovLogFeedback")
    }
}
impl<H, O, S> StateInitializer<S> for CovLogFeedback<H, O> {}
