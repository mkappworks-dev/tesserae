# Client-server from day one, with an integrated server

Network multiplayer is deferred, but the client-server split is not. Single
player runs a Server in the Client's process — Minecraft Java's integrated
server model. Adding multiplayer later is the most expensive retrofit in game
development precisely because single-player code reads and mutates world state
directly, so we pay the split now and add only a transport later.

## Consequences

The Server is the sole authority permitted to mutate a Chunk. Client systems
render and send intent; a Client system that writes world state is a bug even
though single player makes it work. This is the second mistake that will not
look wrong in review.
