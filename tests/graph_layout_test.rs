use alexcode::markdown::compute_graph_layout;

#[test]
fn test_empty_graph() {
    assert!(compute_graph_layout(0, &[], 800.0, 600.0, 300).is_empty());
}

#[test]
fn test_single_node_centered() {
    let pos = compute_graph_layout(1, &[], 800.0, 600.0, 300);
    assert_eq!(pos.len(), 1);
    assert!((pos[0].x - 400.0).abs() < 1e-6);
    assert!((pos[0].y - 300.0).abs() < 1e-6);
}

#[test]
fn test_all_positions_within_bounds() {
    let edges = vec![(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)];
    let pos = compute_graph_layout(4, &edges, 800.0, 600.0, 300);
    assert_eq!(pos.len(), 4);
    for p in pos {
        assert!(p.x >= 0.0);
        assert!(p.x <= 800.0);
        assert!(p.y >= 0.0);
        assert!(p.y <= 600.0);
    }
}

#[test]
fn test_deterministic() {
    let edges = vec![(0, 1), (1, 2)];
    let a = compute_graph_layout(3, &edges, 500.0, 500.0, 300);
    let b = compute_graph_layout(3, &edges, 500.0, 500.0, 300);
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        assert!((a[i].x - b[i].x).abs() < 1e-6);
        assert!((a[i].y - b[i].y).abs() < 1e-6);
    }
}

#[test]
fn test_connected_nodes_closer_than_unconnected() {
    let edges = vec![(0, 1)];
    let pos = compute_graph_layout(3, &edges, 600.0, 600.0, 300);
    assert_eq!(pos.len(), 3);
    let dist = |a: &alexcode::markdown::GraphPoint, b: &alexcode::markdown::GraphPoint| {
        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
    };
    let d01 = dist(&pos[0], &pos[1]);
    let d02 = dist(&pos[0], &pos[2]);
    let d12 = dist(&pos[1], &pos[2]);
    assert!(d01 < d02);
    assert!(d01 < d12);
}

#[test]
fn test_ignores_out_of_range_edges() {
    let edges = vec![(0, 99), (1, 1)];
    let pos = compute_graph_layout(2, &edges, 400.0, 400.0, 300);
    assert_eq!(pos.len(), 2);
}
