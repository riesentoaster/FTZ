use std::{
    iter::{Filter, Map},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
pub enum Direction<T> {
    Outgoing(T),
    Incoming(T),
}

impl<T> Direction<T> {
    pub fn map<U>(self, mapper: fn(T) -> U) -> Direction<U> {
        match self {
            Direction::Outgoing(e) => Direction::Outgoing(mapper(e)),
            Direction::Incoming(e) => Direction::Incoming(mapper(e)),
        }
    }

    #[allow(unused)]
    pub fn outer_to_string(&self) -> &str {
        match self {
            Direction::Outgoing(_) => "Outgoing",
            Direction::Incoming(_) => "Incoming",
        }
    }

    pub fn inner(self) -> T {
        match self {
            Direction::Outgoing(e) => e,
            Direction::Incoming(e) => e,
        }
    }
}

#[allow(unused)]
pub trait DirectionIteratorExt<T>: Iterator<Item = Direction<T>> + Sized {
    /// Applies a mapper function to the content of each Direction item.
    fn map_content<O>(
        self,
        mapper: impl Fn(&T) -> O,
    ) -> Map<Self, impl FnMut(Direction<T>) -> Direction<O>> {
        self.map(move |direction| match direction {
            Direction::Outgoing(e) => Direction::Outgoing(mapper(&e)),
            Direction::Incoming(e) => Direction::Incoming(mapper(&e)),
        })
    }

    fn filter_content(
        self,
        filterer: impl Fn(&T) -> bool,
    ) -> Filter<Self, impl FnMut(&Direction<T>) -> bool> {
        self.filter(move |direction| filterer(direction.deref()))
    }
}

// Implement the trait for all iterators that yield Direction<T>
impl<I, T> DirectionIteratorExt<T> for I where I: Iterator<Item = Direction<T>> {}

impl<T> Deref for Direction<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Direction::Outgoing(e) => e,
            Direction::Incoming(e) => e,
        }
    }
}

impl<T> DerefMut for Direction<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Direction::Outgoing(e) => e,
            Direction::Incoming(e) => e,
        }
    }
}
