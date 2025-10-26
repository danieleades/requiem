# Cycle Detection

> **Note**: Cycle detection is **planned but not yet implemented**. This chapter describes how the feature will work when available.

Cycle detection identifies circular dependencies in requirement graphs, which are usually errors that break valid traceability hierarchies.

## What are Cycles?

A cycle occurs when requirements form a circular dependency chain:

```
USR-001 → SYS-001 → SYS-002 → USR-001
```

Requirements form a loop: USR-001 depends on SYS-001, which depends on SYS-002, which depends back on USR-001.

### Why Cycles are Problematic

**Break hierarchy assumptions**:
- Requirements should form a Directed Acyclic Graph (DAG)
- Cycles violate DAG properties

**Logical impossibility**:
- A cannot be satisfied until B is satisfied
- B cannot be satisfied until C is satisfied
- C cannot be satisfied until A is satisfied
- Nothing can be satisfied!

**Review complexity**:
- Changes propagate indefinitely
- Impossible to determine impact

**Compliance issues**:
- Many standards require acyclic traceability
- Auditors expect clear hierarchies

## Example Cycles

### Simple Cycle (2 requirements)

```
USR-001 → SYS-001 → USR-001
```

**Why it happens**: Accidental bi-directional linking.

**Detection**: SYS-001 lists USR-001 as parent; USR-001 lists SYS-001 as parent.

### Complex Cycle (3+ requirements)

```
USR-001 → SYS-001 → SYS-002 → SWR-001 → USR-001
```

**Why it happens**: Accumulation of links over time without oversight.

**Detection**: Following parent chain eventually returns to starting requirement.

### Self-Reference

```
USR-001 → USR-001
```

**Why it happens**: Data entry error.

**Detection**: Requirement lists itself as a parent.

## Planned Functionality

### Cycle Detection Command

```bash
req check-cycles
```

**Output** (if cycles found):
```
Error: Cycles detected in requirement graph

Cycle 1 (length 3):
  USR-001 → SYS-001 → SYS-002 → USR-001

Cycle 2 (length 2):
  SYS-005 → SWR-003 → SYS-005

Cycle 3 (self-reference):
  SWR-012 → SWR-012

Total cycles: 3

Fix these cycles to ensure valid traceability.
```

**Output** (no cycles):
```
No cycles detected. Requirement graph is acyclic.
```

### Detailed Cycle Information

```bash
req check-cycles --detailed
```

**Output**:
```
Cycle 1:
  Path: USR-001 → SYS-001 → SYS-002 → USR-001
  Length: 3
  Requirements involved:
    - USR-001: User authentication
      Parent: SYS-002 (creates cycle)
    - SYS-001: Authentication service
      Parent: USR-001
    - SYS-002: Session management
      Parent: SYS-001

  Suggested fix:
    Remove parent link from USR-001 to SYS-002
    (User requirements should not depend on system requirements)
```

### Visualization

Generate cycle diagrams:

```bash
req check-cycles --visualize > cycles.dot
dot -Tpng cycles.dot -o cycles.png
```

**Output**: Graph highlighting cyclic paths in red.

## How Detection Works

### Algorithm: Depth-First Search

Requiem will use DFS with cycle detection:

1. **Start from each requirement**
2. **Follow parent links** (or child links for reverse traversal)
3. **Track visited requirements** in current path
4. **If revisit a requirement in current path**: Cycle detected
5. **Report cycle**: Full path from requirement back to itself

### Example Walkthrough

Requirements:
```
USR-001 → SYS-001
SYS-001 → SYS-002
SYS-002 → USR-001  (creates cycle)
```

**Detection**:
```
1. Start at USR-001
2. Visit SYS-001 (parent of USR-001)
3. Visit SYS-002 (parent of SYS-001)
4. Visit USR-001 (parent of SYS-002)
5. USR-001 is already in path → Cycle detected!
6. Report: USR-001 → SYS-001 → SYS-002 → USR-001
```

### Performance

- **Time complexity**: O(V + E) where V = requirements, E = links
- **Expected runtime**: Milliseconds for 1000s of requirements
- **Parallel detection**: Possible for disconnected subgraphs

## Use Cases

### Use Case 1: Continuous Validation

**Scenario**: Automatically check for cycles in CI.

**Workflow**:

```yaml
# .github/workflows/requirements.yml (planned)
- name: Check for cycles
  run: |
    req check-cycles
    if [ $? -ne 0 ]; then
      echo "Cycles detected!"
      req check-cycles --detailed
      exit 1
    fi
```

**Benefit**: Cycles are caught before merging.

### Use Case 2: Fixing Legacy Requirements

**Scenario**: Inherited a requirement set with unknown quality.

**Goal**: Find and fix all cycles.

**Workflow**:

```bash
# Detect cycles
req check-cycles --detailed

# Output:
# Cycle 1: USR-001 → SYS-001 → SYS-002 → USR-001
# Suggested fix: Remove link from USR-001 to SYS-002

# Fix cycle
vim USR-001.md  # Remove SYS-002 from parents

# Verify
req check-cycles
# No cycles detected
```

### Use Case 3: Incremental Checking

**Scenario**: Adding a new link between requirements.

**Goal**: Ensure the new link doesn't create a cycle.

**Workflow**:

```bash
# Add link
req link SWR-001 SYS-005

# Check if cycle created
req check-cycles

# If cycle:
# Error: Cycle detected: SYS-005 → ... → SWR-001 → SYS-005

# If no cycle:
# No cycles detected
```

**Benefit**: Immediate feedback; don't commit cycle-inducing links.

## Breaking Cycles

### Strategy 1: Remove Link

Identify the "weakest" link in the cycle and remove it:

```
USR-001 → SYS-001 → SYS-002 → USR-001
                               ^^^^^^^
                        Remove this link
```

**Criteria for "weakest"**:
- Violates hierarchy (e.g., user requirement depending on system requirement)
- Accidental or incorrect
- Least impactful to remove

### Strategy 2: Reverse Hierarchy

If requirements are at wrong levels, reassign:

```
Before (cycle):
  USR-001 → SYS-001
  SYS-001 → USR-002
  USR-002 → USR-001

After (fixed):
  USR-001, USR-002 → SYS-001
  (SYS-001 now has both as parents; no cycle)
```

### Strategy 3: Introduce Intermediate Requirement

Break cycle by adding a requirement:

```
Before (cycle):
  USR-001 → SYS-001 → USR-001

After (fixed):
  USR-001 → SYS-001 → SYS-002
  (SYS-002 is new, breaks cycle)
```

### Strategy 4: Merge Requirements

If cycle indicates redundancy, merge:

```
Before (cycle):
  REQ-A → REQ-B → REQ-A

After (merged):
  REQ-AB (combines A and B; no cycle)
```

## Configuration (Planned)

### Enable/Disable Cycle Checking

```toml
# config.toml (planned)
[cycles]
# Enable cycle detection
enabled = true

# Fail on cycle detection (exit with error)
fail_on_cycle = true

# Report self-references separately
detect_self_references = true
```

### CI Integration

```toml
[cycles.ci]
# Run cycle detection in CI
enabled = true

# Block merge if cycles detected
block_on_cycle = true
```

## Exceptions (Planned)

In rare cases, cycles might be intentional. Allow marking exceptions:

```yaml
# USR-001.md (hypothetical)
---
_version: '1'
uuid: ...
parents:
- uuid: ...
  hrid: SYS-002
  cycle_exception: true  # Mark as intentional cycle
---
```

Or in config:

```toml
[cycles.exceptions]
# Allow specific cycles
allowed = [
    ["USR-001", "SYS-001", "USR-001"],
]
```

**Use sparingly**: Cycles are almost always errors.

## Related Features

### Topological Sort

After ensuring no cycles, enable topological sorting:

```bash
req sort --topological
```

**Output**: Requirements in dependency order (parents before children).

**Use case**: Determine implementation order.

### Dependency Depth

Analyze requirement depth in hierarchy:

```bash
req depth USR-001
```

**Output**:
```
USR-001:
  Depth: 0 (root requirement)
  Max descendant depth: 4

  Path to deepest descendant:
    USR-001 → SYS-001 → SWR-003 → TST-012
```

## Workarounds (Until Implemented)

Manual cycle detection:

### Script to Detect Simple Cycles

```python
#!/usr/bin/env python3
import glob
import yaml
import re

def parse_requirement(path):
    with open(path) as f:
        content = f.read()
    match = re.match(r'^---\n(.*?)\n---', content, re.DOTALL)
    if match:
        frontmatter = yaml.safe_load(match.group(1))
        return frontmatter
    return None

def find_cycles():
    # Build graph
    graph = {}
    for req_file in glob.glob("*.md"):
        hrid = req_file.replace('.md', '')
        frontmatter = parse_requirement(req_file)
        if frontmatter:
            parents = frontmatter.get('parents', [])
            parent_hrids = [p['hrid'] for p in parents]
            graph[hrid] = parent_hrids

    # Detect cycles with DFS
    def visit(req, path):
        if req in path:
            cycle = path[path.index(req):] + [req]
            print(f"Cycle detected: {' → '.join(cycle)}")
            return True
        if req not in graph:
            return False
        for parent in graph[req]:
            if visit(parent, path + [req]):
                return True
        return False

    for req in graph:
        visit(req, [])

if __name__ == '__main__':
    find_cycles()
```

Run:
```bash
python detect-cycles.py
```

### Manual Inspection

For small requirement sets, manually trace parent chains:

1. Pick a requirement (e.g., USR-001)
2. Follow parent links
3. Track visited requirements
4. If you return to the starting requirement, cycle exists

## Summary

**Key concepts**:

- **Cycle**: Circular dependency chain in requirement graph
- **Problem**: Breaks hierarchy, causes logical impossibility
- **Detection**: Depth-first search with path tracking
- **Resolution**: Remove links, reverse hierarchy, introduce intermediate requirements

**Planned functionality**:
- Automatic cycle detection
- Detailed cycle reports with suggested fixes
- Visualization of cyclic paths
- CI/CD integration
- Configurable checking

**Use cases**:
- CI validation
- Legacy requirement cleanup
- Incremental validation

**Timeline**: Implementation planned for future release

## Next Steps

- Use [workarounds](#workarounds-until-implemented) for manual cycle detection
- Plan your [cycle detection requirements](#configuration-planned) for when feature is available
- See [Coverage Reports](./coverage.md) for traceability analysis
