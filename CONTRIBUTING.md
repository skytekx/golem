<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
# Contributing to Golem

This file is the rulebook of a change: how it is pushed, signed off and written up, what style its
code and its prose follow, how it is tested and which dependencies it may take. Every rule here
holds for every change, by a human or an agent, and a reviewer points at the rule a change breaks.
The rules follow the ones of [Aosabi](https://github.com/netsnek/aosabi/blob/main/CONTRIBUTING.md).

## 1. Push every commit at once

A commit is pushed the same minute it is made. This is the most important rule of the repository.
A dev environment ends without warning: a cloud container is reclaimed, a session times out, or
the usage of an agent runs out in the middle of the work. Whatever is only local at that moment is
gone.

- Commit often, at every step that works or that would hurt to redo, not only at the end of a task.
- Work happens on a work branch of one person or one agent, pushed after every commit. Before a
  merge into `main` the branch is cleaned into commits of section 3, and only then may it be force
  pushed, with `--force-with-lease` and on that branch alone.
- A push that fails because of the network is retried at once and then with growing pauses. A push
  that still fails is the first thing to report, before any other work goes on. A push that origin
  refuses is reported at once, since no retry mends it.
- An agent ends no session and no answer with an unpushed commit or uncommitted changes. Before it
  stops it checks that `git status` is clean and that `git log @{u}..` is empty.

The hook `tools/hooks/post-commit` pushes after every commit, and `tools/hooks/commit-msg` checks
the message of every `git commit` with the lint of section 3. Both are switched on with
`git config core.hooksPath tools/hooks`. Git runs no commit-msg hook for `git revert` and
`git cherry-pick`, so they run with `--no-commit`, and a `git commit` after them writes the message.

## 2. Signed-off-by

Every commit carries a `Signed-off-by` line under the
[Developer Certificate of Origin 1.1](https://developercertificate.org/), which certifies that the
change may be submitted under the license of the repository. The line names the holder of Golem,
also on a commit that an AI agent writes:

```text
Signed-off-by: Florian Kleber <opensource@skytekx.com>
```

The author of a commit is the person or the account that made it. An agent that helped names
itself in a `Co-Authored-By` line above the sign off, and the line may also be spelled
`Co-authored-by`, as GitHub writes it in the commits that it makes. A commit without the sign off
is not merged.

## 3. Commit messages

A commit message is a [Conventional Commit](https://www.conventionalcommits.org/) as
[Commitizen](https://commitizen-tools.github.io/commitizen/) writes it, in the form of the Linux
kernel's `Documentation/process/submitting-patches.rst`: a typed subject, a body of prose and the
trailers.

```text
feat(essentials): refuse amounts below one cent in pay

The pay command must never move money from the receiver to the sender.
Golem parses every amount as whole cents and refuses anything that is
not above zero before it touches an account.

Closes: https://github.com/skytekx/golem/issues/12
Signed-off-by: Florian Kleber <opensource@skytekx.com>
```

- The subject is `type(scope): summary`, imperative, starting in lower case, with no full stop at
  its end and at most 72 characters long. A breaking change writes `type(scope)!: summary`.
- The type is one of `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore` and
  `revert`. The subjects that git writes itself, `fixup!`, `squash!` and `amend!` of
  `git commit --fixup` and `Revert "..."` of `git revert`, are none of them and are refused. A
  correction of an earlier commit is a commit with a subject of its own until the branch is
  cleaned, and a revert is a `revert(scope): summary`, written with `git revert --no-commit` and a
  `git commit`, since `git revert` commits without the hook of section 1.
- The scope is the crate without its `golem-` prefix, such as `jvm` or `permissions`, the bridge
  mod as `bridge`, or a top level folder or topic, such as `tests`, `github`, `okf` or `license`.
- An empty line follows, then a body wrapped at 72 columns that explains the problem first, then
  why this change solves it, then how. The diff already shows what changed, so the body is no list
  of files. A line that holds a URL alone, a trailer of the last paragraph or an indented log line
  may be wider.
- The trailers close the message in the kernel's form, one `Token: value` per line, all of them in
  the last paragraph, and the last of them is `Signed-off-by`. A fix of an earlier commit carries
  `Fixes: <12 characters of the hash> ("<subject>")` with the hash and the subject of a commit of
  the repository, as `git log -1 --abbrev=12 --format='%h ("%s")' <commit>` prints them. A commit
  that solves an issue carries `Closes:` with the URL of the issue, and `Link:` points to further
  discussion. `Reported-by`, `Reviewed-by`, `Tested-by`, `Suggested-by` and `Co-developed-by` are
  used as the kernel uses them.
- One logical change per commit. On `main` every commit builds and passes the notebooks on its own,
  so that `git bisect` works. A refactor never rides along with a change of behaviour.

[.cz.toml](.cz.toml) gives Commitizen the subject, so `cz commit -- -s` writes a message through
its questions and `cz check` checks one. The questions of the body and of the trailers take several
lines, which Esc and then Enter end. The commit lint,
[.github/scripts/commit_lint.py](.github/scripts/commit_lint.py), checks the rules of the kernel
that Commitizen does not know. CI runs both on the commits of every push and pull request: the
commits that a pull request adds to its base, the commits that a push adds to `main`, and on
another branch or a tag the commits that `main` does not hold.

## 4. Style

The code follows the coding style of the Linux kernel, as Aosabi writes it down in its
[coding style](https://github.com/netsnek/aosabi/blob/main/Documentation/coding-style.md).

- Every file opens with its SPDX lines, see
  [Documentation/licensing.md](Documentation/licensing.md), and a source file then with a block
  comment that says what the file is.
- Rust is formatted by `rustfmt` and passes `clippy` with warnings as errors on every commit.
- Every `unsafe` block, which the JNI host needs, carries a `// SAFETY:` comment that says why it is
  sound, as the kernel requires. Unsafe code lives in the crates that talk to the JVM and nowhere
  else.
- The Java of the bridge mod takes the same rules where they fit: tabs, K&R braces, lines of at
  most 100 columns, short functions and no clever tricks.
- Comments say why, not what. Names are plain words, not abbreviations.

Prose in the repository, in comments, documentation, the wiki and commit messages, is English in
the present tense and says what is, never how it came to be. It uses no dashes as punctuation, no
semicolons and no exclamation marks. People are they and them. Markdown is wrapped at 100 columns.
A measured number names how it was measured: the notebook, the machine and the day.

The docs lint, [.github/scripts/docs_lint.py](.github/scripts/docs_lint.py), holds the Markdown of
the wiki, of `Documentation/`, this file and README.md to these rules of punctuation, pronouns and
width. It also checks that the index of the wiki links every page and that
[Documentation/testing.md](Documentation/testing.md) names every notebook of the matrix. CI runs
it on every push and every pull request, and `python3 -I .github/scripts/docs_lint.py` runs it on
a tree.

## 5. Tests are notebooks

Every test of Golem is a Jupyter notebook in Python under [tests/notebooks/](tests/notebooks/), run
by papermill, as [Documentation/testing.md](Documentation/testing.md) describes. The workspace holds
no `#[test]` and no doc test that asserts behaviour, and a change adds none.

- A change that fixes or adds behaviour adds or extends the notebook cell that proves it.
- The internals are reached through `golem-devtool`, one primitive per subcommand, so a notebook
  checks one primitive at a time.
- A notebook has a `parameters` cell that papermill overrides, and it captures the output of every
  process it drives, so a failing run explains itself.
- Executed notebooks land under `tests/notebooks/out/` and are never committed, and a notebook is
  committed with its outputs stripped.

## 6. Quirks and security issues

The port of the plugins fixes what they do wrong today instead of copying it. Every quirk and every
security issue found while porting gets its own issue in `skytekx/golem` before the fix, with the
label `quirk` or `security`, the old behaviour with its evidence and the behaviour Golem has
instead. A pull request that ports or changes a plugin lists every issue it fixes in a section of
its body, one `Fixes #n` per issue, or says that it found none. As long as the live server runs
EssentialsX and the repository is public, a security issue that is exploitable there is filed as a
private security advisory instead of a public issue.

## 7. The wiki

The [OKF wiki](okf/index.md) holds the design and the operation of Golem. A change that alters
behaviour, the configuration, the API, a ported command or the data of a plugin updates the
matching page in the same pull request, and every quirk and security issue gets a line on the page
of its plugin with the link to its issue.

## 8. Dependencies

Every crate from outside the project is code that ships in Golem and a part of its supply chain, so
a dependency is a weighing, in this order:

1. The standard library first, then code written in the project, then Aosabi.
2. An outside crate only where it is more secure than own code, clearly faster, or needed to talk
   to something Golem cannot reach otherwise, such as the JVM or SQLite.
3. Then only a crate that is well known, well maintained and licensed compatibly with the AGPL 3.

A pull request that adds an outside crate says why it meets the rule. Telemetry is never added.

## 9. Nothing private

No credential, token, key, private host name, internal address, player data or mail address of a
third party enters the repository, in any file, an output cell of a notebook included. The only
addresses of others are these two, each where a license or a rule of this file requires it.

- The address in the copyright line of the upstream of a port, which the license of the upstream
  asks Golem to keep. It stands in NOTICE, in `ports.toml`, in the SPDX lines of every file of the
  port, as rule 9 of the license lint requires, and in the cases of that lint.
- The address of an agent in its `Co-Authored-By` line of section 2, in a commit message and in the
  cases of the commit lint.

An address under a domain that RFC 2606 reserves for examples, such as `example.org`, names nobody
and may stand in a case of a notebook.
