//! Hierarchical clustering, for ordering the rows and columns of a heatmap.

/// How the distance between two clusters is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Linkage {
    /// Mean distance between members (UPGMA).
    Average,
    Centroid,
    /// Distance between the closest members.
    Nearest,
    /// Distance between the furthest members.
    Furthest,
    Ward,
}

impl Linkage {
    pub const ALL: [Linkage; 5] = [
        Linkage::Average,
        Linkage::Centroid,
        Linkage::Nearest,
        Linkage::Furthest,
        Linkage::Ward,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Linkage::Average => "Average neighbour (UPGMA)",
            Linkage::Centroid => "Centroid",
            Linkage::Nearest => "Nearest neighbour",
            Linkage::Furthest => "Furthest neighbour",
            Linkage::Ward => "Ward",
        }
    }
}

/// One merge of two clusters. Clusters `0..n` are the original items;
/// cluster `n + i` is the result of merge `i`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Merge {
    pub left: usize,
    pub right: usize,
    /// Distance at which the two clusters were joined.
    pub height: f64,
}

/// A clustering of `n` items.
#[derive(Debug, Clone, PartialEq)]
pub struct Dendrogram {
    pub merges: Vec<Merge>,
    /// The items in the order of the dendrogram's leaves.
    pub order: Vec<usize>,
}

impl Dendrogram {
    pub fn max_height(&self) -> f64 {
        self.merges.iter().map(|m| m.height).fold(0.0, f64::max)
    }
}

/// Cluster items by the Euclidean distance between their vectors. Returns
/// `None` for fewer than two items or vectors of differing length.
pub fn cluster(items: &[Vec<f64>], linkage: Linkage) -> Option<Dendrogram> {
    let n = items.len();
    if n < 2 || items.iter().any(|v| v.len() != items[0].len()) {
        return None;
    }
    let mut condensed = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in i + 1..n {
            let squared: f64 = items[i]
                .iter()
                .zip(&items[j])
                .map(|(a, b)| (a - b) * (a - b))
                .sum();
            condensed.push(squared.sqrt());
        }
    }
    let method = match linkage {
        Linkage::Average => kodama::Method::Average,
        Linkage::Centroid => kodama::Method::Centroid,
        Linkage::Nearest => kodama::Method::Single,
        Linkage::Furthest => kodama::Method::Complete,
        Linkage::Ward => kodama::Method::Ward,
    };
    let result = kodama::linkage(&mut condensed, n, method);
    let merges: Vec<Merge> = result
        .steps()
        .iter()
        .map(|s| Merge {
            left: s.cluster1,
            right: s.cluster2,
            height: s.dissimilarity,
        })
        .collect();

    // Leaf order: depth-first from the last merge, left branch first.
    let mut order = Vec::with_capacity(n);
    let mut stack = vec![n + merges.len() - 1];
    while let Some(cluster) = stack.pop() {
        if cluster < n {
            order.push(cluster);
        } else {
            let merge = merges[cluster - n];
            stack.push(merge.right);
            stack.push(merge.left);
        }
    }
    Some(Dendrogram { merges, order })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Vec<f64>> {
        vec![vec![0.0], vec![10.0], vec![1.0], vec![11.0], vec![30.0]]
    }

    #[test]
    fn close_items_are_adjacent_in_the_leaf_order() {
        for linkage in Linkage::ALL {
            let d = cluster(&items(), linkage).unwrap();
            assert_eq!(d.merges.len(), 4, "{linkage:?}");
            let position = |item: usize| d.order.iter().position(|&x| x == item).unwrap();
            assert_eq!(position(0).abs_diff(position(2)), 1, "{linkage:?}");
            assert_eq!(position(1).abs_diff(position(3)), 1, "{linkage:?}");
            let mut sorted = d.order.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, [0, 1, 2, 3, 4]);
        }
    }

    // scipy.cluster.hierarchy.linkage(pdist(x), 'average') for the same data.
    #[test]
    fn average_linkage_heights_match_scipy() {
        let d = cluster(&items(), Linkage::Average).unwrap();
        let heights: Vec<f64> = d.merges.iter().map(|m| m.height).collect();
        for (a, e) in heights.iter().zip([1.0, 1.0, 10.0, 24.5]) {
            assert!((a - e).abs() < 1e-12, "{heights:?}");
        }
        assert_eq!(d.max_height(), 24.5);
    }

    #[test]
    fn too_few_items() {
        assert!(cluster(&[vec![1.0]], Linkage::Average).is_none());
        assert!(cluster(&[vec![1.0], vec![1.0, 2.0]], Linkage::Average).is_none());
        assert!(cluster(&[vec![1.0], vec![1.0]], Linkage::Ward).is_some());
    }
}
