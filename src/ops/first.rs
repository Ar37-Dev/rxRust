use crate::{
  ops::{take::TakeOp, Take},
  prelude::*,
};
use std::{cell::RefCell, rc::Rc};

/// emit only the first item emitted by an Observable
pub trait First {
  fn first(self) -> TakeOp<Self>
  where
    Self: Sized + Take,
  {
    self.take(1)
  }
}

impl<'a, O> First for O where O: Subscribable<'a> {}

/// emit only the first item (or a default item) emitted by an Observable
pub trait FirstOr<'a> {
  fn first_or(self, default: Self::Item) -> FirstOrOp<TakeOp<Self>, Self::Item>
  where
    Self: Subscribable<'a>,
  {
    FirstOrOp {
      source: self.first(),
      default: Some(default),
    }
  }
}

impl<'a, O> FirstOr<'a> for O where O: Subscribable<'a> {}

pub struct FirstOrOp<S, V> {
  source: S,
  default: Option<V>,
}

impl<'a, S, T> Subscribable<'a> for FirstOrOp<S, T>
where
  T: 'a,
  S: Subscribable<'a, Item = T>,
{
  type Item = S::Item;
  type Err = S::Err;
  type Unsubscribable = S::Unsubscribable;

  fn subscribe_return_state(
    self,
    next: impl Fn(&Self::Item) -> OState<Self::Err> + 'a,
    error: Option<impl Fn(&Self::Err) + 'a>,
    complete: Option<impl Fn() + 'a>,
  ) -> Self::Unsubscribable {
    let next = Rc::new(next);
    let c_next = next.clone();
    let Self { source, default } = self;
    let default = Rc::new(RefCell::new(default));
    let c_default = default.clone();
    source.subscribe_return_state(
      move |v| {
        c_default.borrow_mut().take();
        c_next(v)
      },
      error,
      Some(move || {
        let default = default.borrow_mut().take();
        if let Some(d) = default {
          next(&d);
        }
        if let Some(ref comp) = complete {
          comp();
        }
      }),
    )
  }
}

#[cfg(test)]
mod test {
  use super::{First, FirstOr};
  use crate::prelude::*;
  use std::cell::Cell;

  #[test]
  fn first() {
    let completed = Cell::new(false);
    let next_count = Cell::new(0);

    let numbers = Subject::<'_, _, ()>::new();
    numbers.clone().first().subscribe_complete(
      |_| next_count.set(next_count.get() + 1),
      || completed.set(true),
    );

    (0..2).for_each(|v| {
      numbers.next(&v);
    });

    assert_eq!(completed.get(), true);
    assert_eq!(next_count.get(), 1);
  }

  #[test]
  fn first_or() {
    let completed = Cell::new(false);
    let next_count = Cell::new(0);
    let v = Cell::new(0);

    let mut numbers = Subject::<'_, i32, ()>::new();
    numbers.clone().first_or(100).subscribe_complete(
      |_| next_count.set(next_count.get() + 1),
      || completed.set(true),
    );

    // normal pass value
    (0..2).for_each(|v| {
      numbers.next(&v);
    });
    assert_eq!(next_count.get(), 1);
    assert_eq!(completed.get(), true);

    completed.set(false);
    numbers
      .clone()
      .first_or(100)
      .subscribe_complete(|value| v.set(*value), || completed.set(true));

    numbers.complete();
    assert_eq!(completed.get(), true);
    assert_eq!(v.get(), 100);
  }
}
