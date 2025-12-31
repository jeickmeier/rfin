# Pull Request Creation Instructions

## Current Status

✅ **All implementation complete**: 6 phases, 21 commits
✅ **All tests passing**: 6155 tests (Rust + WASM + Python)
✅ **Zero warnings**: Clippy, lint, and documentation all clean
✅ **Documentation ready**: CHANGELOG, completion docs, and PR description prepared

**Branch**: `marge-list-d3b5`
**Commits**: 21 commits (from `6c6ff091` to `201a08b0`)

---

## Step 1: Push Branch to Remote

The branch needs to be pushed to GitHub. You'll need to authenticate with GitHub:

```bash
cd /private/var/folders/20/nm1z0qb90hvcnsbr53t87xt40000gn/T/zenflow/worktrees/marge-list-d3b5

# Option 1: Using HTTPS (requires GitHub token)
git push -u origin marge-list-d3b5

# Option 2: Using SSH (if you have SSH keys set up)
git remote set-url origin git@github.com:jeickmeier/rfin.git
git push -u origin marge-list-d3b5
```

**If authentication fails**, you may need to:
1. Generate a GitHub Personal Access Token at https://github.com/settings/tokens
2. Use the token as your password when prompted
3. Or configure SSH keys: https://docs.github.com/en/authentication/connecting-to-github-with-ssh

---

## Step 2: Create Pull Request on GitHub

Once the branch is pushed:

1. **Go to GitHub**: https://github.com/jeickmeier/rfin
2. **You should see a banner**: "Compare & pull request" for branch `marge-list-d3b5`
3. **Click "Compare & pull request"**
4. **Copy the PR description** from `.zenflow/tasks/marge-list-d3b5/PR_DESCRIPTION.md`
5. **Set the title**: `Marge List: Code Consolidation Refactoring`
6. **Paste the description** into the PR body
7. **Add labels** (if applicable):
   - `refactoring`
   - `no-breaking-changes`
   - `high-priority` (optional)
8. **Assign reviewers**:
   - Quant team member (for Phase 2 Monte Carlo changes)
   - Structuring desk member (for Phase 3 waterfall changes)
   - Core maintainer (for overall architecture)
9. **Click "Create pull request"**

---

## Step 3: Link Supporting Documentation

In the PR description or comments, add links to:

- **Specification**: `.zenflow/tasks/marge-list-d3b5/spec.md`
- **Implementation Plan**: `.zenflow/tasks/marge-list-d3b5/plan.md`
- **Phase Completion Docs**:
  - Phase 1: `.zenflow/tasks/marge-list-d3b5/PHASE1_SUMMARY.md`
  - Phase 2: `.zenflow/tasks/marge-list-d3b5/PHASE2_COMPLETE.md`
  - Phase 3: `.zenflow/tasks/marge-list-d3b5/PHASE3_STEP3_COMPLETE.md`
  - Phase 4: `.zenflow/tasks/marge-list-d3b5/PHASE4_STEP3_COMPLETE.md`
  - Phase 5: `.zenflow/tasks/marge-list-d3b5/PHASE5_COMPLETE.md`
  - Phase 6: `.zenflow/tasks/marge-list-d3b5/PHASE6_STEP2_COMPLETE.md`
  - Final Verification: `.zenflow/tasks/marge-list-d3b5/FINAL_VERIFICATION_COMPLETE.md`

---

## Step 4: CI/CD and Review

### Expected CI Results
All CI checks should pass:
- ✅ **Rust tests**: 5799/5799 passing
- ✅ **WASM tests**: 26/26 passing
- ✅ **Python tests**: 330/330 passing
- ✅ **Clippy**: Zero warnings
- ✅ **Lint**: Zero warnings
- ✅ **Documentation**: Builds successfully

### Review Process
1. **Request reviews** from assigned reviewers
2. **Address comments** by pushing additional commits to the branch
3. **Re-run CI** if changes are made
4. **Wait for approval** from all required reviewers

### Review Checklist
Reviewers should verify:
- [ ] All tests pass (6155/6155)
- [ ] Zero clippy warnings
- [ ] Performance within 5% (0% actual regression)
- [ ] Backward compatibility maintained (deprecated APIs work)
- [ ] Migration guides are clear
- [ ] Documentation is complete
- [ ] CHANGELOG is updated

---

## Step 5: Merge

Once all reviews are approved and CI passes:

1. **Choose merge strategy**:
   - **Squash and merge** (recommended): Creates a single commit with all changes
   - **Merge commit**: Preserves all 21 individual commits
   - **Rebase and merge**: Replays commits on top of main

2. **Update commit message** (if squashing):
   ```
   Marge List: Code Consolidation Refactoring (#XXX)

   Reduces code duplication by 500+ lines through unified abstractions:
   - Phase 1: Market data curve restoration (327→80 lines)
   - Phase 2: Monte Carlo payoff consolidation
   - Phase 3: Parameter reduction via context structs (15→2-3 params)
   - Phase 4: Trait-based market data extraction
   - Phase 5: Waterfall execution unification
   - Phase 6: JSON envelope boilerplate removal

   All changes maintain 100% backward compatibility with zero behavioral changes.
   Tests: 6155/6155 passing | Warnings: 0 | Performance: 0% regression
   ```

3. **Delete branch** after merge (optional, but recommended)

---

## Troubleshooting

### Push Fails with Authentication Error
```bash
fatal: could not read Username for 'https://github.com': Device not configured
```

**Solution**: Set up GitHub authentication:
```bash
# Option 1: Use HTTPS with token
git config --global credential.helper store
git push -u origin marge-list-d3b5
# Enter your GitHub username and token when prompted

# Option 2: Use SSH
git remote set-url origin git@github.com:jeickmeier/rfin.git
git push -u origin marge-list-d3b5
```

### CI Tests Fail
1. **Check logs** in GitHub Actions
2. **Run tests locally**: `make test-rust && make test-wasm && make test-python`
3. **Fix issues** and push new commits to the branch
4. **CI will automatically re-run** on new commits

### Merge Conflicts
If main branch has changed since you started:
```bash
git checkout marge-list-d3b5
git fetch origin
git merge origin/main
# Resolve conflicts if any
git push origin marge-list-d3b5
```

### Need to Make Changes After PR Created
Just commit and push to the same branch:
```bash
# Make your changes
git add .
git commit -m "Address review comments: ..."
git push origin marge-list-d3b5
```

---

## Post-Merge Checklist

After the PR is merged:

- [ ] Verify main branch CI passes
- [ ] Monitor production for any unexpected behavior
- [ ] Announce deprecations to relevant teams
- [ ] Consider scheduling a tech talk on refactoring techniques
- [ ] Update internal documentation with new patterns
- [ ] Plan gradual migration timeline for deprecated APIs

---

## Success Metrics to Track

After merge, track these metrics over the next sprint:

### Performance
- Attribution calculation time (should be ±5%)
- Monte Carlo pricing speed (should be ±5%)
- Waterfall execution time (should be ±5%)

### Correctness
- Attribution P&L differences (<1bp tolerance)
- Monte Carlo price stability
- Waterfall conservation law violations (should be 0)

### Adoption
- Teams migrating to new APIs
- Questions/issues raised about deprecated APIs
- Documentation clarity feedback

---

## Rollback Instructions (Emergency Only)

If critical issues arise in production:

```bash
# Revert the merge commit
git revert -m 1 <merge-commit-sha>
git push origin main

# Or reset to before the merge (more destructive)
git reset --hard <commit-before-merge>
git push --force origin main
```

**Only use if**:
- Attribution P&L differs by >1bp
- Monte Carlo prices outside tolerances
- Waterfall distributions fail conservation checks
- Performance regression >10%
- Production crashes

---

## Questions?

For help with:
- **Git/GitHub**: Check GitHub docs or ask DevOps team
- **CI/CD**: Check Actions logs or ask DevOps team
- **Code changes**: Review phase completion docs
- **Migration**: Check PR description migration guide

---

## Quick Commands Reference

```bash
# Push branch
git push -u origin marge-list-d3b5

# Create PR from CLI (using gh CLI)
gh pr create --title "Marge List: Code Consolidation Refactoring" --body-file .zenflow/tasks/marge-list-d3b5/PR_DESCRIPTION.md

# Check PR status
gh pr view marge-list-d3b5

# Merge PR from CLI
gh pr merge marge-list-d3b5 --squash --delete-branch

# Monitor CI
gh run list --branch marge-list-d3b5

# View PR in browser
gh pr view marge-list-d3b5 --web
```

**Note**: `gh` CLI requires installation: `brew install gh` (macOS) or https://cli.github.com/

---

**Ready to create the PR? Follow the steps above!** 🚀
