use core::ops;

use std::collections::BTreeMap;

use semilattice::{GuardedPair, Map, Max, Redactable, SemiLattice, Set};

use crate::{ActorID, MessageID, Owned, Patchset, Reaction, Root, Shared, Tag};

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
#[cbor(transparent)]
pub struct Vote<const N: usize>(#[n(0)] Map<ActorID, Max<u64>>);

impl<const N: usize> ops::Deref for Vote<N> {
    type Target = Map<ActorID, Max<u64>>;

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

        for v in self.values() {
            // modulo for arbitrary N isn't efficient, but if N is always a
            // power of two, this becomes a bit-mask. Any excess values could
            // be reserved or may be considered equivalent to the highest
            // element.
            res[v.0 as usize % N] += 1;
        }

        res
    }
}

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
struct Thread {
    #[n(0)]
    titles: GuardedPair<Max<u64>, Set<String>>,
    #[n(1)]
    tags: Map<Tag, Vote<4>>,
}

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
struct Comment {
    #[n(0)]
    reply_to: Set<MessageID>,
    #[n(1)]
    content: Map<u64, Redactable<String>>,
    #[n(2)]
    reactions: Map<Reaction, Vote<2>>,
    #[n(3)]
    backrefs: Set<MessageID>,
    #[n(4)]
    commits: Map<u64, Set<Patchset>>,
}

#[derive(Default, Debug, Clone, SemiLattice, PartialEq, minicbor::Encode, minicbor::Decode)]
pub struct Detailed {
    #[n(0)]
    threads: Map<ActorID, Map<u64, Thread>>,
    #[n(1)]
    messages: Map<ActorID, Map<u64, Comment>>,
}

impl SemiLattice<Root> for Detailed {
    fn join(mut self, other: Root) -> Self {
        for (actor, slice) in other.inner.inner {
            let threads = self.threads.entry_mut(actor.clone());

            for (
                id,
                Owned {
                    titles,
                    reply_to,
                    content,
                    commits,
                },
            ) in slice.owned.inner
            {
                if titles.value.len() > 0 {
                    threads.entry_mut(id).titles.join_assign(titles);
                }
                for br in &*reply_to {
                    self.messages
                        .entry_mut(br.0.clone())
                        .entry_mut(br.1)
                        .backrefs
                        .insert((actor.clone(), id));
                }
                self.messages
                    .entry_mut(actor.clone())
                    .entry_mut(id)
                    .join_assign(Comment {
                        reply_to,
                        content,
                        reactions: Map::default(),
                        backrefs: Set::default(),
                        commits,
                    });
            }

            for ((aid, id), Shared { tags, reactions }) in slice.shared.inner {
                self.messages
                    .entry_mut(aid.clone())
                    .entry_mut(id)
                    .reactions
                    .join_assign(
                        reactions
                            .inner
                            .into_iter()
                            .map(|(r, v)| (r, Vote(Map::singleton(actor.clone(), v))))
                            .collect::<BTreeMap<_, _>>()
                            .into(),
                    );

                if tags.len() > 0 {
                    self.threads
                        .entry_mut(aid.clone())
                        .entry_mut(id)
                        .tags
                        .join_assign(
                            tags.inner
                                .into_iter()
                                .map(|(r, v)| (r, Vote(Map::singleton(actor.clone(), v))))
                                .collect::<BTreeMap<_, _>>()
                                .into(),
                        );
                }
            }
        }

        self
    }
}

impl Detailed {
    pub fn display(&self) {
        // An awful example UI.

        for (aid, thread) in &self.threads.inner {
            for (id, Thread { titles, tags }) in &thread.inner {
                println!("Author: {:?} [{}]", aid, id);
                for title in &titles.value.inner {
                    println!("Title: {}", title);
                }

                let mut tag_votes = BTreeMap::new();
                for (tag, votes) in &tags.inner {
                    let va = votes.aggregate();
                    *tag_votes.entry(tag).or_insert(0) += va[1] as i64 - va[2] as i64;
                }

                print!("Tags: ");
                for (tag, score) in tag_votes.into_iter().filter(|(_, x)| *x > 0) {
                    print!("{} ({}), ", tag, score);
                }
                println!();
                println!();

                let mut stack = vec![(0, (aid.clone(), *id))];

                while let Some((depth, (aid, id))) = stack.pop() {
                    let message = self
                        .messages
                        .inner
                        .get(&aid)
                        .expect("Expected aid")
                        .get(&id)
                        .expect("Expected id.");

                    stack.extend(
                        message
                            .backrefs
                            .inner
                            .clone()
                            .into_iter()
                            .map(|x| (depth + 1, x)),
                    );

                    println!("Depth: {}", depth);
                    println!("Author: {:?} [{}]", aid, id);
                    for (version, content) in &message.content.inner {
                        println!("Body [{}]: {:?}", version, content);
                    }
                    for (reaction, votes) in &message.reactions.inner {
                        println!("Reaction [{}]: {:?}", reaction, votes);
                    }
                    println!();
                }
            }
        }
    }
}
