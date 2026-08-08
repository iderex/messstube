# Decision records

One file per decision. A record says what was decided, why, what it cost, what
was rejected, and what would show the decision to be wrong.

The template is `TEMPLATE.md` in this directory. It sits here rather than
elsewhere because the person about to write a record is already looking at this
directory, and a template kept away from the thing it templates is a template
nobody finds.

## Numbering

Records are numbered from `0001` with a four-digit prefix and a short slug:
`0007-hostile-input-budget.md`.

A number is taken by the record that lands with it and is never reused. Numbers
are never reassigned and never closed up. If `0004` were withdrawn tomorrow,
`0005` would not become `0004`, because every reference to a record anywhere
would then point at a different decision without changing.

Numbers can therefore have gaps, and a gap is not a defect. Records are also not
required to land in numerical order; several of these arrived together and the
numbers were assigned by the issues that raised them rather than by the order the
files were written.

The number is assigned by the issue that raises the decision. That issue names
the exact filename in its done-condition, which is what stops two branches
choosing the same number.

## Supersession, not deletion

A record is never deleted and never rewritten to say something else.

When a decision is overturned, a new record is written. It states what changed,
what evidence changed it, and which record it replaces. The old record stays in
place and gains one line near the top naming the record that replaced it. That
line is the only edit a landed record takes.

Deleting or rewriting the old one destroys the only account of why the project
thought what it thought, which is the part worth reading two years later. A
record that was wrong is still evidence about how the question was approached,
and the correction is worth more when the reader can see both.

The record being superseded is not softened while it is being superseded. If it
admitted a cost, the admission stays; if it named the observation that would
reverse it, that sentence stays and the new record says whether that is what
happened.

What is on `main` today:

    $ ls docs/decisions/0*.md | wc -l
    12
    $ grep -l -i 'superseded by' docs/decisions/0*.md ; echo "exit=$?"
    exit=1

Twelve records and none of them superseded.

## What a record is not

It is not a proposal. A record lands after the decision is taken, and the
argument happens on the issue that raised it.

It is not documentation of how the software works. A record explains why the
software is shaped the way it is. Somebody who wants to use the tool should not
have to read one.

It is not permanent because it is written down. The last field of every record
names the observation that would reverse it, and a record whose author could not
name one had not finished thinking.
