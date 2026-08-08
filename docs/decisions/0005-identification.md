# 0005. Identification is content-based, and ambiguity is a refusal

Decided 2026-08-08. Raised in #6.

## What is being decided

Given a file and a set of readers, which reader gets it. Every route into this
library goes through that question, so the answer is fixed before any reader
exists rather than settling into whatever the first one happened to do.

## The decision

Identification is content-based. Each reader declares a recognition predicate
over the leading bytes of the file and its total length, and that predicate is
the only evidence that can make a reader the answer.

The file name extension is a hint. It may order the candidates so that the
likely reader is tried first, and it may never be the only evidence for a claim.
A reader whose predicate does not claim the bytes is not selected because the
name suggested it, and a reader whose predicate does claim the bytes is not
skipped because the name did not.

Both failure directions are refusals rather than guesses.

If more than one reader claims the file, identification fails and the failure
names every reader that claimed it. It does not pick one.

If no reader claims the file, identification fails and says so. It does not fall
back to trying readers in turn to see which one does not error.

## The maximum prefix a recognition predicate may read

A recognition predicate may read at most the first 4096 bytes of the file, and
the file's total length. It may read fewer. It may not read more, and it may not
seek beyond that window, which is what makes the bound a property of the
interface rather than a convention each reader keeps or forgets.

4096 bytes, because it is large enough for the header of every family in scope
as those families are understood today, and because it is one page on the
platforms this runs on, so the bound costs one read of the underlying file
however many readers are asked. A predicate that genuinely needs more bytes than
that to tell one format from another is describing an ambiguity between the two
formats, and the answer to that is the refusal above rather than a deeper look.

The number is here rather than in each reader so that there is one place to
argue with. Raising it is a change to this record with a reason, not a constant
somebody edits.

## The reasons

Extensions in this field carry almost no information. `.dat` and `.bin` are used
by at least four of the families in scope, one vendor reuses a single extension
across incompatible generations of its own instrument, and files arrive renamed
by whoever copied them off the machine. An identification scheme resting on the
name is resting on the one part of the file that anybody may have changed.

Trying every reader until one does not fail is worse than useless. A lenient
reader succeeds on a foreign file and returns numbers that look plausible, and
plausible wrong numbers are the single most damaging thing a project like this
can produce. The person downstream has no way to notice, because the failure has
already been converted into data.

Refusing an ambiguous file is a small inconvenience for whoever hit it, and it is
recoverable: they can see which readers claimed the file and say which one they
meant. Returning the wrong instrument's data silently is not recoverable at all,
because nobody knows it happened.

Bounding the predicate keeps identification cheap and keeps it out of the attack
surface. Identifying a directory of large files should cost a page per file, and
the code that runs before anything is known about the input is the code with the
least context to defend itself, so it is the code that should be doing the least.

## What it costs

Two readers whose predicates overlap turn a file that one of them could have read
into a refusal. That is accepted, and the repair is to tighten a predicate so it
stops claiming what it cannot read.

The repair is explicitly not a preference order between readers. A preference
order would make the answer depend on registration order, which is a property of
this repository's build rather than of the file in front of it. Two installations
of the same library would then disagree about the same bytes, and neither would
be wrong.

The cost falls hardest on formats that genuinely do not identify themselves in
their first bytes. A format with no magic and no structural signature in the
prefix cannot be recognised under this rule and will be refused. That is the
correct answer here: it is a format this library cannot tell apart from another,
and saying so is better than choosing.

## What was rejected and why

Extension-first identification, rejected because the extension is the part of the
file most likely to have been changed by somebody who was not thinking about it.

Try-every-reader, rejected because its failure mode is silent wrong numbers.

A preference order to break ambiguity, rejected because it makes the answer
depend on registration order rather than on the file.

An unbounded predicate, rejected because it makes identification expensive on
directories and makes the least-informed code in the library the code that reads
the most.

A confidence score per reader, with the highest score winning. Rejected because
it is a preference order with arithmetic in front of it, and because the number
would be invented by each reader's author against no shared scale.

## What would reverse it

A format in scope that cannot be identified from a bounded prefix and that
matters enough to be worth the exception. The reversal is not a general one: it
is a named format, with a record saying what it needs and what the exception
costs, rather than a relaxation of the rule for everything.

Repeated refusals in ordinary use would also reverse the 4096-byte figure, and
that is the cheaper half of this record to change. Moving the bound leaves the
rest of the decision standing.
