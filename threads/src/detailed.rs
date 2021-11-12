use core::ops;

use std::collections::BTreeMap;

use semilog::{MapLattice, Max, Redactable, Semilattice, SetLattice, VecLattice};

use crate::{ActorID, MessageID, Owned, Patchset, Reaction, Root, Shared, Slice, Tag};

#[derive(Default, Debug, Clone, Semilattice, PartialEq, minicbor::Encode, minicbor::Decode)]
#[cbor(transparent)]
pub struct Vote<const N: usize>(#[n(0)] MapLattice<ActorID, Max<u64>>);

impl<const N: usize> ops::Deref for Vote<N> {
    type Target = MapLattice<ActorID, Max<u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> ops::DerefMut for Vote<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> Vote<N> {
    pub fn aggregate(&self) -> [usize; N] {
        let mut res = [0; N];

        for (_, v) in &self.inner {
            // modulo for arbitrary N isn't efficient, but if N is always a
            // power of two, this becomes a bit-mask. Any excess values could
            // be reserved or may be considered equivalent to the highest
            // element.
            res[v.0 as usize % N] += 1;
        }

        res
    }
}

#[derive(Default, Debug, Clone, Semilattice, PartialEq, minicbor::Encode, minicbor::Decode)]
struct Comment {
    #[n(0)]
    titles: VecLattice<SetLattice<String>>,
    #[n(1)]
    content: VecLattice<Redactable<String>>,
    #[n(2)]
    responses: SetLattice<MessageID>,
    #[n(3)]
    tags: MapLattice<Tag, Vote<4>>,
    #[n(4)]
    reactions: MapLattice<Reaction, Vote<2>>,
    #[n(5)]
    commits: VecLattice<SetLattice<Patchset>>,
}

#[derive(Default, Debug, Clone, Semilattice, PartialEq, minicbor::Encode, minicbor::Decode)]
pub struct Detailed {
    #[n(0)]
    threads: SetLattice<MessageID>,
    #[n(1)]
    comments: MapLattice<ActorID, VecLattice<Comment>>,
}

impl Detailed {
    pub fn join_root(mut self, other: Root) -> Self {
        for (actor, Slice { owned, shared }) in other.inner.inner {
            for (
                id,
                Owned {
                    titles,
                    content,
                    commits,
                },
            ) in owned.inner.into_iter().enumerate()
            {
                let id = id as u64;
                if !titles.is_empty() {
                    self.threads.insert((actor.clone(), id));
                }

                self.comments
                    .entry_mut(&actor)
                    .entry_mut(id)
                    .join_assign(Comment {
                        titles,
                        content,
                        reactions: MapLattice::default(),
                        responses: SetLattice::default(),
                        tags: MapLattice::default(),
                        commits,
                    });
            }

            for (aid, comments) in shared.inner {
                for (
                    id,
                    Shared {
                        tags,
                        reactions,
                        responses,
                    },
                ) in comments.inner
                {
                    self.comments
                        .entry_mut(&aid)
                        .entry_mut(id)
                        .join_assign(Comment {
                            reactions: MapLattice::from_iter(reactions.iter().map(|(r, v)| {
                                (r.clone(), Vote(MapLattice::singleton(actor.clone(), *v)))
                            })),
                            tags: MapLattice::from_iter(tags.iter().map(|(r, v)| {
                                (r.clone(), Vote(MapLattice::singleton(actor.clone(), *v)))
                            })),
                            responses: SetLattice::from_iter(
                                responses.iter().map(|id| (actor.clone(), id.0)),
                            ),
                            ..Default::default()
                        });
                }
            }
        }

        self
    }
}

impl Detailed {
    // An awful example UI.
    pub fn display(&self) {
        let mut stack = Vec::new();

        for (mid, _) in &**self.threads {
            stack.clear();
            stack.push((0, mid));

            while let Some((depth, id)) = stack.pop() {
                let comment = self
                    .comments
                    .entry(&id.0)
                    .expect("Expected aid")
                    .entry(id.1)
                    .expect("Expected id.");

                stack.extend(comment.responses.into_iter().map(|x| (depth + 1, x)));

                println!("Depth: {}", depth);
                println!("Author: {:?} [{}]", id.0, id.1);

                let mut tag_votes = BTreeMap::new();
                for (tag, votes) in &*comment.tags {
                    let va = votes.aggregate();
                    *tag_votes.entry(tag).or_insert(0) += va[1] as i64 - va[2] as i64;
                }

                print!("Tags: ");
                for (tag, score) in tag_votes.into_iter().filter(|(_, x)| *x > 0) {
                    print!("{}, ({}), ", tag, score);
                }
                println!();

                for (version, content) in comment.content.iter().enumerate() {
                    println!("Body [{}]: {:?}", version, content);
                }
                print!("Reactions: ");
                for (reaction, votes) in &*comment.reactions {
                    print!("{} ({:?})", reaction, votes);
                }
                println!();
                println!();
            }

            println!("---");
        }
    }
}
