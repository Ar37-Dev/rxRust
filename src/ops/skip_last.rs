use crate::prelude::*;
use observable::observable_proxy_impl;
use observer::error_proxy_impl;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct SkipLastOp<S> {
  pub(crate) source: S,
  pub(crate) count: usize,
}

#[doc(hidden)]
macro observable_impl($subscription:ty, $($marker:ident +)* $lf: lifetime) {
  fn actual_subscribe<O: Observer<Self::Item, Self::Err> + $($marker +)* $lf>(
    self,
    subscriber: Subscriber<O, $subscription>,
  ) -> Self::Unsub {
    let subscriber = Subscriber {
      observer: SkipLastObserver {
        observer: subscriber.observer,
        count: self.count,
        queue: VecDeque::new(),
      },
      subscription: subscriber.subscription,
    };
    self.source.actual_subscribe(subscriber)
  }
}

observable_proxy_impl!(SkipLastOp, S);

impl<'a, Item: 'a, S> LocalObservable<'a> for SkipLastOp<S>
where
  S: LocalObservable<'a, Item = Item>,
{
  type Unsub = S::Unsub;
  observable_impl!(LocalSubscription, 'a);
}

impl<S> SharedObservable for SkipLastOp<S>
where
  S: SharedObservable,
  S::Item: Send + Sync + 'static,
{
  type Unsub = S::Unsub;
  observable_impl!(SharedSubscription, Send + Sync + 'static);
}

pub struct SkipLastObserver<O, Item> {
  observer: O,
  count: usize,
  queue: VecDeque<Item>,
}

impl<Item, Err, O> Observer<Item, Err> for SkipLastObserver<O, Item>
where
  O: Observer<Item, Err>,
{
  fn next(&mut self, value: Item) { self.queue.push_back(value); }

  error_proxy_impl!(Err, observer);
  fn complete(&mut self) {
    if self.count <= self.queue.len() {
      let skip_index = self.queue.len() - self.count;
      for value in self.queue.drain(..skip_index) {
        self.observer.next(value);
      }
    }
    self.observer.complete();
  }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn base_function() {
    let mut completed = false;
    let mut ticks = vec![];

    observable::from_iter(0..10)
      .skip_last(5)
      .subscribe_complete(|v| ticks.push(v), || completed = true);

    assert_eq!(ticks, vec![0, 1, 2, 3, 4]);
    assert_eq!(completed, true);
  }

  #[test]
  fn base_empty_function() {
    let mut completed = false;
    let mut ticks = vec![];

    observable::from_iter(0..10)
      .skip_last(11)
      .subscribe_complete(|v| ticks.push(v), || completed = true);

    assert_eq!(ticks, vec![]);
    assert_eq!(completed, true);
  }

  #[test]
  fn skip_last_support_fork() {
    let mut nc1 = 0;
    let mut nc2 = 0;
    {
      let skip_last5 = observable::from_iter(0..100).skip_last(5);
      let f1 = skip_last5.clone();
      let f2 = skip_last5;

      f1.skip_last(5).subscribe(|_| nc1 += 1);
      f2.skip_last(5).subscribe(|_| nc2 += 1);
    }
    assert_eq!(nc1, 90);
    assert_eq!(nc2, 90);
  }

  #[test]
  fn into_shared() {
    observable::from_iter(0..100)
      .skip_last(5)
      .skip_last(5)
      .to_shared()
      .subscribe(|_| {});
  }
}
