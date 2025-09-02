use std::collections::HashSet;

use linked_hash_map::LinkedHashMap;
use linked_hash_set::LinkedHashSet;

use crate::{run::dependency_resolution::TopologicalSortError, task::ResolvedTaskInvocation};

/// Performs topological sort on the dependency graph to determine execution order
pub fn topological_sort(
    graph: &LinkedHashMap<ResolvedTaskInvocation, LinkedHashSet<ResolvedTaskInvocation>>,
) -> Result<Vec<ResolvedTaskInvocation>, TopologicalSortError> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new(); // For cycle detection

    // Start with all nodes that have no incoming edges
    for node in graph.keys() {
        if !visited.contains(node) {
            visit_node(node, graph, &mut visited, &mut visiting, &mut result)?;
        }
    }

    // Reverse the result since we built it in reverse topological order
    result.reverse();
    Ok(result)
}

/// Recursive helper function for topological sort using DFS
fn visit_node(
    node: &ResolvedTaskInvocation,
    graph: &LinkedHashMap<ResolvedTaskInvocation, LinkedHashSet<ResolvedTaskInvocation>>,
    visited: &mut HashSet<ResolvedTaskInvocation>,
    visiting: &mut HashSet<ResolvedTaskInvocation>,
    result: &mut Vec<ResolvedTaskInvocation>,
) -> Result<(), TopologicalSortError> {
    if visiting.contains(node) {
        // We've found a cycle - reconstruct the cycle path
        let cycle = reconstruct_cycle(node, graph, visiting);
        return Err(TopologicalSortError::CycleDetected(cycle));
    }

    if visited.contains(node) {
        return Ok(());
    }

    visiting.insert(node.clone());

    if let Some(dependencies) = graph.get(node) {
        for dep in dependencies {
            visit_node(dep, graph, visited, visiting, result)?;
        }
    }

    visiting.remove(node);
    visited.insert(node.clone());
    result.push(node.clone());

    Ok(())
}

/// Reconstructs a cycle path for error reporting
fn reconstruct_cycle(
    start: &ResolvedTaskInvocation,
    graph: &LinkedHashMap<ResolvedTaskInvocation, LinkedHashSet<ResolvedTaskInvocation>>,
    visiting: &HashSet<ResolvedTaskInvocation>,
) -> Vec<ResolvedTaskInvocation> {
    let mut cycle = vec![start.clone()];
    let mut current = start;

    // Find a path back to the start node
    while let Some(dependencies) = graph.get(current) {
        for dep in dependencies {
            if visiting.contains(dep) {
                cycle.push(dep.clone());
                if dep == start {
                    return cycle;
                }
                current = dep;
                break;
            }
        }
    }

    cycle
}
