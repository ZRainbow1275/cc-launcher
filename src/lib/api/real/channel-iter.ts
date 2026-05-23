import { Channel } from "@tauri-apps/api/core";

export interface ChannelStreamHandle<TEvent, TResult> {
  channel: Channel<TEvent>;
  iterable: AsyncIterable<TEvent>;
  done: Promise<TResult>;
}

export function makeChannelStream<TEvent, TResult>(
  start: (channel: Channel<TEvent>) => Promise<TResult>,
  isTerminal: (evt: TEvent) => boolean,
): ChannelStreamHandle<TEvent, TResult> {
  const channel = new Channel<TEvent>();
  const queue: TEvent[] = [];
  const waiters: Array<(v: IteratorResult<TEvent>) => void> = [];
  let closed = false;

  const flushClose = () => {
    closed = true;
    while (waiters.length) {
      waiters.shift()!({ value: undefined as never, done: true });
    }
  };

  channel.onmessage = (msg) => {
    queue.push(msg);
    while (queue.length && waiters.length) {
      const next = queue.shift()!;
      waiters.shift()!({ value: next, done: false });
      if (isTerminal(next)) flushClose();
    }
  };

  const done = start(channel).finally(flushClose);

  const iterable: AsyncIterable<TEvent> = {
    [Symbol.asyncIterator](): AsyncIterator<TEvent> {
      return {
        next(): Promise<IteratorResult<TEvent>> {
          if (queue.length) {
            const value = queue.shift()!;
            if (isTerminal(value)) flushClose();
            return Promise.resolve({ value, done: false });
          }
          if (closed) {
            return Promise.resolve({
              value: undefined as never,
              done: true,
            });
          }
          return new Promise((resolve) => waiters.push(resolve));
        },
      };
    },
  };

  return { channel, iterable, done };
}
