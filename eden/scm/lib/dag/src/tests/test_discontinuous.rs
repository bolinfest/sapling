/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Tests about discontinuous segments
//!
//! Previously, segments in a group are continuous. In other words, all segments
//! in the master group can be represented using a single span `0..=x`.  With
//! discontinuous segments, a group might be represented as a few spans.
//!
//! The discontinuous spans are designed to better support multiple long-lived
//! long branches. For example:
//!
//! ```plain,ignore
//! 1---2---3--...--999---1000     branch1
//!      \
//!       5000--...--5999---6000   branch2
//! ```
//!
//! Note: discontinuous segments is not designed to support massive amount of
//! branches. It introduces O(branch) factor in complexity in many places.

use crate::ops::DagAlgorithm;
use crate::ops::DagPersistent;
use crate::tests::DrawDag;
use crate::tests::TestDag;
use crate::Group;
use crate::Vertex;
use crate::VertexListWithOptions;
use crate::VertexOptions;

#[tokio::test]
async fn test_simple_3_branches() {
    let mut dag = TestDag::new();
    let draw = DrawDag::from(
        r#"
            A--B--C--D--E--F
                   \
                    G--H--I
                     \
                      J--K--L"#,
    );

    let heads = VertexListWithOptions::from(vec![
        reserved_head("F", 100),
        reserved_head("I", 50),
        reserved_head("L", 0),
    ]);

    dag.dag.add_heads_and_flush(&draw, &heads).await.unwrap();
    assert_eq!(
        format!("{:?}", &dag.dag),
        r#"Max Level: 0
 Level 0
  Group Master:
   Next Free Id: 162
   Segments: 3
    J+159 : L+161 [G+106]
    G+106 : I+108 [C+2]
    A+0 : F+5 [] Root OnlyHead
  Group Non-Master:
   Next Free Id: N0
   Segments: 0
"#
    );

    assert_eq!(
        format!("{:?}", dag.dag.ancestors("I".into()).await.unwrap()),
        "<spans [G:I+106:108, A:C+0:2]>"
    );
    assert_eq!(
        format!("{:?}", dag.dag.descendants("B".into()).await.unwrap()),
        "<spans [J:L+159:161, G:I+106:108, B:F+1:5]>"
    );
    assert_eq!(
        format!("{:?}", dag.dag.range("C".into(), "K".into()).await.unwrap()),
        "<spans [J:K+159:160, G+106, C+2]>"
    );
    assert_eq!(
        format!("{:?}", dag.dag.parents("G".into()).await.unwrap()),
        "<spans [C+2]>"
    );
    assert_eq!(
        format!("{:?}", dag.dag.children("C".into()).await.unwrap()),
        "<spans [G+106, D+3]>"
    );

    let all = dag.dag.all().await.unwrap();
    assert_eq!(
        format!("{:?}", dag.dag.children(all.clone()).await.unwrap()),
        "<spans [J:L+159:161, G:I+106:108, B:F+1:5]>"
    );
    assert_eq!(
        format!("{:?}", dag.dag.parents(all.clone()).await.unwrap()),
        "<spans [J:K+159:160, G:H+106:107, A:E+0:4]>"
    );
    assert_eq!(
        format!(
            "{:?}",
            dag.dag.range(all.clone(), all.clone()).await.unwrap()
        ),
        "<spans [J:L+159:161, G:I+106:108, A:F+0:5]>"
    );
}

fn reserved_head(s: &'static str, reserve_size: u32) -> (Vertex, VertexOptions) {
    (
        Vertex::from(s),
        VertexOptions {
            reserve_size,
            highest_group: Group::MASTER,
            ..Default::default()
        },
    )
}
