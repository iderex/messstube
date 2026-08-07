# 0002. A library, one command-line tool, and where a language binding sits

Decided 2026-08-07. Raised in #3.

## What is being decided

What an operator gets and what a programmer gets. The two are settled together
because an interface cannot be designed until it is known who calls it.

## The decision

The library is the product. One command-line tool is the operator surface. Both
live in this repository. There is no graphical application and none is planned.
A Python binding is designed for and is not shipped in the first release.

Whether the absence of a graphical surface is a permanent boundary or the
current one is entry 5 of #1, which belongs to the maintainer. If it is settled
the other way, that answer arrives as a record superseding this one rather than
as a change to this one.

Designed for a binding means three constraints on the public interface, and they
hold from the first reader rather than from the day a binding is attempted:

1. It returns plain owned data rather than borrowed views.
2. It keeps lifetimes out of its signatures.
3. Its error values are describable without Rust vocabulary.

## The reasons

The readers are the value, and locking them inside an application is the failure
this board exists in reaction to. Several of the formats in scope are readable
today only by opening a graphical program, which is why they are unreadable in a
pipeline, on a server, and in ten years. A library that a command-line tool
happens to use is the shape that stays useful in all three places.

One tool rather than several keeps the operator surface small enough to document
honestly. A surface nobody can document completely is one whose gaps are
discovered by the operator instead of by the project.

The three constraints above are what make a binding an addition later rather
than a redesign. Each of them is cheap to hold while the interface is being
written and expensive to retrofit once readers depend on it.

## What it costs

Deferring the binding costs the audience most likely to want these readers. Until
it exists they have to run the tool and read its output. That is workable because
the tool writes plain text, and it belongs in the documentation as a stated
limitation rather than as something a reader discovers.

Shipping the binding in the first release would have cost a wheel-building
pipeline across three operating systems, which roughly doubles the release
surface before there is a single verified reader to put in it. That is the
trade, and it is taken in the direction of having something verified to ship.

## What was rejected and why

A graphical application, rejected because it is the shape that made these
formats unreadable outside one program in the first place.

Several command-line tools rather than one, rejected because the operator surface
then grows faster than the documentation that has to cover it.

Shipping the Python binding in the first release, rejected for the release
surface above. Not designing for it, rejected because the retrofit is a redesign.

## What would reverse it

A reader from this repository that nobody outside it ever calls. That is the
observation that says the product surface was chosen for the wrong audience, and
it is visible without asking anyone: the library is public, and whether anything
depends on it is a fact about the world rather than an opinion about the plan.
