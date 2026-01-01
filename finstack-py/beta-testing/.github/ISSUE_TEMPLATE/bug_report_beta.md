---
name: Beta Bug Report
about: Report a bug found during beta testing
title: '[BETA BUG] '
labels: beta-feedback, bug
assignees: ''
---

## 🐛 Bug Description

**Brief Summary**:
A clear and concise description of what the bug is.

**Severity**:
- [ ] Critical - Blocks beta testing or causes data corruption
- [ ] High - Significantly impacts functionality
- [ ] Medium - Workaround available
- [ ] Low - Minor issue or cosmetic

## 📝 Reproduction Steps

**Steps to reproduce the behavior**:
1. Import '...'
2. Create '...'
3. Call method '...'
4. See error

**Minimal Reproducible Example**:
```python
# Paste minimal code that reproduces the bug
import finstack

# Your code here...
```

## 🎯 Expected Behavior

What you expected to happen:

## ❌ Actual Behavior

What actually happened:

**Error Message** (if applicable):
```
Paste full error message and stack trace here
```

## 🖥️ Environment

**Operating System**:
- [ ] macOS (version: _____)
- [ ] Linux (distribution: _____)
- [ ] Windows (version: _____)

**Python Version**: (e.g., 3.12.1)
```bash
python --version
```

**Finstack Version**: (e.g., 1.0.0b1)
```bash
python -c "import finstack; print(finstack.__version__)"
```

**Installation Method**:
- [ ] Wheel (pip install)
- [ ] Source (maturin develop)
- [ ] Test PyPI

**Dependencies**:
```bash
pip list | grep -E "finstack|polars|pandas|numpy"
```

## 📋 Additional Context

**Workaround** (if found):
Describe any workaround you've found.

**Related Issues**:
Link to any related issues or discussions.

**Screenshots** (if applicable):
Add screenshots to help explain your problem.

**Sample Data** (if applicable):
If the bug is data-dependent, provide minimal sample data.

## 🔍 Investigation

**What have you tried?**:
- [ ] Reinstalled package
- [ ] Tested on different Python version
- [ ] Tested with fresh virtual environment
- [ ] Checked documentation
- [ ] Searched existing issues

**Additional Notes**:
Any other information that might be helpful in diagnosing the issue.

---

**Beta Tester Information** (optional):
- Name: _____
- Organization: _____
- Persona: [ ] Portfolio Manager [ ] Credit Analyst [ ] Quantitative Analyst [ ] Risk Manager [ ] Data Engineer

---

**For Maintainers**:
- [ ] Bug confirmed
- [ ] Root cause identified
- [ ] Fix implemented
- [ ] Test added
- [ ] Documentation updated (if needed)
- [ ] Included in beta 2 (if needed)
