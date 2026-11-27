# 24: Every river ends in still water, and ponds

Status: ready-for-agent
Blocked by: 23

Spec: [Water that looks natural](../spec.md), Lakes. From the user's review:
rivers should end in a lake, a pond or the sea.

## What to build

A shallow basin a stream enters, 1 m deep or more, holds a pond instead of
being filled flat, so a stream ends in still water. Where a river crosses a
window's edge, the fine solve takes the coarse solve's catchment there, so it
has one width on both sides. Small ponds of 3 to 20 m, a hashed candidate per
128 m cell, stand at their rim's lowest point and need no lip; one also serves
as a spring pool.

## Acceptance criteria

- [ ] Every stream ends in a lake, a pond or the sea, or leaves what is
      stored; a test walks every stream.
- [ ] A river's width agrees on both sides of a region border; a test.
- [ ] Ponds on the map; a capture shown to the user.

## Comments
