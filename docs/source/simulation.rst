Match Simulation in Openfootmanager
===================================

Simulating a football match is not an easy task.

There are numerous different approaches and many things to consider
while trying to get it right.

In Openfootmanager, the main focus is not to get an accurate and
fully-fledged 3D or 2D match simulation, at least not yet.
Simulation is hard, and implementing these things into the core of
the game takes a long time, especially for a one-man effort.

In the future OFM might even try to implement such features, but
that is beyond the scope of the first release. Everything starts small,
and OFM is no different than any other software out there.

The game, as of now, does not plan to use any third-party library (aside
from the GUI library), so we are starting from scratch with everything that the
Python standard library provides.

In this document we are going to see the concepts for match simulation efforts
in OFM, and what the approach we will be taking to simulate a match.

Previous attempts
=================

I am also working in a very similar project, that
is currently more advanced feature-wise and is closer now to its first alpha release.
The eSports Manager (github.com/sturdy-robot/esports-manager) project had
a very similar issue when dealing with simulation.

The approach used in eSM was the first thing I have tried in OFM,
but it quickly became apparent that although this could easily produce nice results,
it would not allow for player interaction during the game simulation, and I found
many problems when trying to make it fit OFM.

For specific implementation details:

In eSM there is a class called MobaEventHandler. This Event handler class takes
care of all events that can happen during a MOBA match.

In a MOBA match, some events just cannot happen depending on what time you are
into the game. You can't simply have a tower falling at 2 minutes into the game
because that just does not happen in professional MOBA matches.

So the EventHandler has to watch for certain conditions and keep track of
events that are allowed to happen at certain times during the game. In a football game
that just is not the case. A player can get injured, suffer a foul, get a red card,
yellow card, score a goal, get a corner kick, etc. at any point in the game.

For that kind of approach to work you would have to list out every single possible
event and assign them fixed probabilities according to the game. But that is very hard to do.
These probabilities could be weighted according to each team's skill, but how do you
even balance that? It is indeed a trial and error effort, just like it was for eSM.

But another thing makes this approach even harder. Unlike a MOBA game, where you can't
just replace a player during the game, football allows you to do such a thing. So at one point
you would have to break the simulation loop to interfere into the game, and the current eSM
implementation does not allow for that.

Also, a key point that was implemented into that kind of simulation is the concept of points.
Points were bonuses given to players in the eSM match simulation whenever they accomplished
a significant event in the match. Points are basically a representation of gold in a MOBA match:
the more gold you have, the stronger you get in the game.

However, that simply does not work in soccer. Even if a team has scored many goals during a match,
that does not mean that they will necessarily get stronger than their opponents, and we
would solely have to account to the player's skill level and current stamina to balance the
outcome of events in the game.

Well, what I mean is, the approach used in eSM could potentially work here, but it would definitely
not be the best approach for a match simulation. I recognize that maybe we could make the match
simulation work with this approach, but I want to try something more robust.

Looking for inspiration
=======================

WIP

