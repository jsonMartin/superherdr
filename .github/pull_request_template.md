## Change and purpose

Describe the problem, resulting behavior, and scope. For a proposal-only PR, describe the proposed behavior and the decision you are asking for.

## Contribution path

State the path: direct fix or improvement, proposal only, single-PR proposal implementation, or implementation of a separately accepted proposal. See [CONTRIBUTING.md](https://github.com/jsonMartin/superherdr/blob/main/CONTRIBUTING.md). For a direct PR, explain why it needs no proposal.

For a proposal implementation, give the OpenSpec change path, the accepted revision, who accepted it, and a link to the acceptance.

## Verification and limitations

List the checks you ran and their results, and anything left untested. Include reproduction and regression evidence for bugs, measurements for performance claims, and screenshots or recordings for UI changes. For proposal-only PRs, report `openspec validate --strict` and open questions.

## Checklist

Mark items that do not apply with a reason.

- [ ] I followed the contribution path required by CONTRIBUTING.md.
- [ ] If a proposal was required, it was validated and accepted before implementation.
- [ ] Any material change to the accepted scope was accepted again before implementation.
- [ ] I reported focused checks, results, and limitations.
- [ ] I updated affected documentation and specs, or explained why none were needed.
- [ ] If this completes a proposal, I finished its tasks, archived the change, and ran `just spec-check`.
