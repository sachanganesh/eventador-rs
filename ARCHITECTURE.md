# Architecture

## Eventador

This struct wraps the `Arc<RingBuffer>` and offers public sync/async APIs to
publish and subscribe to events.

## RingBuffer

This is the core data structure that stores events. It contains a `Sequencer`
that atomically assigns a publishable event to a slot in the ring. Each event
slot is an `EventWrapper`.

### Sequencer

The Sequencer has a monotonically increasing count of the number of events
that have been published thus far. A publisher must complete a challenge to
acquire write-access to an `EventWrapper`, and this challenge is determined
by both the declared `WaitStrategy` and a CAS loop.

The sequence number assigned to the publisher is the sequence number for the
event, and is also mapped to a specific location on the ring.

### EventWrapper

This is a type-alias for `CachePadded<Arc<EventEnvelope>>`.

### EventEnvelope

This structure tracks the event sequence number, and the actual event message.
The envelope atomically replaces the event message inside on publishing, and
updates the sequence number accordingly.

This structure also tracks the number of subscribers that are waiting to read
the next event to be written, and their wake-up handles.

## Subscriber

This structure has an internal `Sequence` counter, which is atomically
incremented after it reads an event. The sequence number of the subscriber
indicates what event it needs to read next.

The subscriber registers its handle with the `EventEnvelope` to wake it again
when the event becomes readable.

An event is readable if the internal sequence number of the subscriber matches
the internal sequence number of the event envelope. An event will be ignored if
it is not the same type that the subscriber is subscribed to.

All subscribers walk the entirety of the ring at this point in time, though
this can be optimized.

## Publish

Publishing an event involves:

1. Successfully completing the posed challenge from the `Sequencer`
2. Overwriting the event in the envelope with the new one
3. Updating the envelope's sequence number to be the same as the event's
4. Waking all waiting subscribers

## WaitStrategy

These are policies that enable the `Sequencer` to behave in different ways
when subscribers are lagging behind publishers. As there is a bounded number of
`EventWrapper`s in the ring, the user decides how and when a publisher can
overwrite an event that has not yet been read by all subscribers.