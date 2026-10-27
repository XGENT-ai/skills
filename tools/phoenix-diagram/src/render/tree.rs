use super::{escape_text, TypeRenderer};
use crate::geometry::{format_number as number, rounded_path};
use crate::layout::tree::{STACKED, TOGGLE_R};
use crate::scene::{Card, CompiledDiagram, DiagramFacts, Node, Text};
use std::collections::HashMap;
use std::fmt::Write;

pub(super) const RENDERER: TypeRenderer = TypeRenderer {
    root_attributes,
    defs,
    body,
    legend_glyph,
};

// Each top-level branch owns one hue its whole subtree inherits; the hue is
// paint only, while the role (root, branch, leaf) stays in the node details.
const SUBTREE_HUES: [&str; 6] = [
    "backend",
    "database",
    "cloud",
    "frontend",
    "messagebus",
    "external",
];

fn direction(scene: &CompiledDiagram) -> &str {
    match &scene.facts {
        DiagramFacts::Tree { direction, .. } => direction,
        _ => "down",
    }
}

fn root_attributes(scene: &CompiledDiagram) -> String {
    // Node kinds are colour slots for root, branch and leaf, not authored types.
    let mut attributes = format!(
        " data-tree-ui=\"\" data-tree-direction=\"{}\" data-node-kind-derived=\"\"",
        escape_text(direction(scene))
    );
    if scene.reader_fit == "intrinsic-height" {
        attributes.push_str(" data-reader-min-text=\"7.5\"");
    }
    attributes
}

fn defs(_scene: &CompiledDiagram) -> String {
    let mut style = String::from("<style>svg[data-tree-ui] .tree-hue-root { --tree-hue: var(--text-muted); --tree-wash: transparent; }");
    for hue in SUBTREE_HUES {
        write!(style, "svg[data-tree-ui] .tree-hue-{hue} {{ --tree-hue: var(--{hue}-stroke); --tree-wash: var(--{hue}-fill); }}").unwrap();
    }
    style.push_str(concat!(
        "svg[data-tree-ui] .tree-edge { stroke-linejoin: round; stroke-linecap: round; }",
        "svg[data-tree-ui] .tree-edge-trunk { stroke: color-mix(in srgb, var(--text) 18%, var(--arrow)); }",
        "svg[data-tree-ui] .tree-edge-hued { stroke: color-mix(in srgb, var(--tree-hue) 42%, var(--arrow)); }",
        "svg[data-tree-ui] .tree-root { fill: var(--text); stroke: none; }",
        "svg[data-tree-ui] [data-tree-depth=\"0\"] .tree-label { fill: var(--mask); }",
        "svg[data-tree-ui] [data-tree-depth=\"0\"] .tree-sublabel { fill: var(--mask); opacity: 0.68; }",
        "svg[data-tree-ui] .tree-branch { fill: var(--tree-wash); stroke: color-mix(in srgb, var(--tree-hue) 48%, var(--lane-stroke)); }",
        "svg[data-tree-ui] .tree-leaf { fill: var(--mask); stroke: color-mix(in srgb, var(--tree-hue) 22%, var(--lane-stroke)); }",
        "svg[data-tree-ui] .tree-socket { fill: var(--mask); stroke: var(--tree-hue); pointer-events: none; }",
        "svg[data-tree-ui] .tree-accent { stroke: var(--tree-hue); pointer-events: none; }",
        "svg[data-tree-ui] [data-tree-stack] { display: none; fill: var(--mask); stroke: color-mix(in srgb, var(--tree-hue) 70%, var(--lane-stroke)); }",
        "svg[data-tree-ui] [data-tree-collapsed] [data-tree-stack] { display: inline; }",
        "svg[data-tree-ui] [data-tree-hidden] { display: none; }",
        "svg[data-tree-ui] .tree-toggle-hit { fill: transparent; }",
        "svg[data-tree-ui] .tree-toggle-disc { fill: var(--mask); stroke: color-mix(in srgb, var(--tree-hue) 55%, var(--lane-stroke)); transition: stroke 0.15s ease, fill 0.15s ease; }",
        "svg[data-tree-ui] .tree-toggle-glyph { stroke: var(--text-muted); fill: none; transition: stroke 0.15s ease; }",
        "svg[data-tree-ui] .tree-toggle-count { display: none; }",
        "svg[data-tree-ui] .tree-count-pill { fill: var(--tree-hue); stroke: none; }",
        "svg[data-tree-ui] .tree-toggle-count text { fill: var(--mask); }",
        "svg[data-tree-ui] [data-tree-toggle] { cursor: pointer; outline: none; }",
        "svg[data-tree-ui] [data-tree-toggle][aria-expanded=\"true\"] .tree-toggle-plus { display: none; }",
        "svg[data-tree-ui] [data-tree-toggle][aria-expanded=\"false\"] .tree-toggle-count { display: inline; }",
        "svg[data-tree-ui] [data-tree-toggle][aria-expanded=\"false\"] .tree-toggle-disc { fill: var(--tree-hue); stroke: var(--tree-hue); }",
        "svg[data-tree-ui] [data-tree-toggle][aria-expanded=\"false\"] .tree-toggle-glyph { stroke: var(--mask); }",
        "svg[data-tree-ui] [data-tree-toggle]:hover .tree-toggle-disc, svg[data-tree-ui] [data-tree-toggle]:focus-visible .tree-toggle-disc { stroke: var(--tree-hue); stroke-width: 2; }",
        "svg[data-tree-ui] [data-tree-toggle]:hover .tree-toggle-glyph, svg[data-tree-ui] [data-tree-toggle]:focus-visible .tree-toggle-glyph { stroke: var(--text); }",
        "svg[data-tree-ui] [data-tree-expand-all] { cursor: pointer; outline: none; }",
        "svg[data-tree-ui] [data-tree-expand-all] rect { fill: var(--mask); stroke: var(--lane-stroke); }",
        "svg[data-tree-ui] [data-tree-expand-all]:is(:hover, :focus-visible) rect { stroke: var(--arrow-emphasis); stroke-width: 2; }",
        "svg[data-tree-ui]:not([data-tree-any-collapsed]) [data-tree-expand-all] { display: none; }",
        // Print always shows the complete tree, whether or not a print event fired.
        "@media print { svg[data-tree-ui] [data-tree-hidden] { display: inline; } svg[data-tree-ui] [data-tree-collapsed] [data-tree-stack] { display: none; } }",
        "</style>"
    ));
    style
}

struct Hierarchy<'a> {
    root: &'a str,
    parent: HashMap<&'a str, &'a str>,
    hue: HashMap<&'a str, &'static str>,
}

impl<'a> Hierarchy<'a> {
    fn of(scene: &'a CompiledDiagram) -> Self {
        let parent: HashMap<&str, &str> = scene
            .relations
            .iter()
            .map(|relation| (relation.to.as_str(), relation.from.as_str()))
            .collect();
        let root = match &scene.facts {
            DiagramFacts::Tree { root, .. } => root.as_str(),
            _ => "",
        };
        let mut hierarchy = Self {
            root,
            parent,
            hue: HashMap::new(),
        };
        let top: Vec<&str> = scene
            .relations
            .iter()
            .filter(|relation| relation.from == root)
            .map(|relation| relation.to.as_str())
            .collect();
        for node in &scene.nodes {
            let id = node.authored.id.as_str();
            let lineage: Vec<&str> = std::iter::once(id).chain(hierarchy.ancestors(id)).collect();
            if let Some(index) = lineage
                .iter()
                .find_map(|item| top.iter().position(|branch| branch == item))
            {
                hierarchy
                    .hue
                    .insert(id, SUBTREE_HUES[index % SUBTREE_HUES.len()]);
            }
        }
        hierarchy
    }

    fn ancestors(&self, id: &str) -> Vec<&'a str> {
        let mut chain = Vec::new();
        let mut current = self.parent.get(id).copied();
        while let Some(parent) = current {
            chain.push(parent);
            current = self.parent.get(parent).copied();
        }
        chain
    }

    fn attributes(&self, id: &str) -> String {
        let ancestors = self.ancestors(id);
        if ancestors.is_empty() {
            String::new()
        } else {
            format!(
                " data-tree-ancestors=\"{}\"",
                escape_text(&ancestors.join(" "))
            )
        }
    }

    fn hue_class(&self, id: &str) -> String {
        self.hue
            .get(id)
            .map_or_else(|| "tree-hue-root".into(), |hue| format!("tree-hue-{hue}"))
    }

    fn has_children(&self, id: &str) -> bool {
        self.parent.values().any(|parent| *parent == id)
    }
}

fn step(depth: usize) -> usize {
    depth.min(12)
}

fn tspans(texts: &[&Text]) -> String {
    texts
        .iter()
        .map(|text| {
            format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                number(text.position.x),
                number(text.position.y),
                escape_text(&text.content)
            )
        })
        .collect()
}

fn write_text(out: &mut String, lines: &[&Text], attributes: &str) {
    if let Some(first) = lines.first() {
        write!(
            out,
            "<text {attributes} x=\"{}\" font-size=\"{}\" text-anchor=\"{}\">{}</text>",
            number(first.position.x),
            number(first.font_size),
            escape_text(&first.anchor),
            tspans(lines)
        )
        .unwrap();
    }
}

fn write_node(
    scene: &CompiledDiagram,
    tree: &Hierarchy<'_>,
    locale: &crate::i18n::Locale,
    node: &Node,
    out: &mut String,
) {
    let id = node.authored.id.as_str();
    let depth = tree.ancestors(id).len();
    let leaf = !tree.has_children(id);
    let is_root = id == tree.root;
    let (tone, role, context, weight) = if is_root {
        ("frontend", "tree-root", "node.context.tree.root", 750)
    } else if leaf {
        ("external", "tree-leaf", "node.context.tree.leaf", 560)
    } else {
        ("backend", "tree-branch", "node.context.tree.branch", 700)
    };
    let context = locale.text(context, &[]);
    let sublabel = node.authored.facts["sublabel"]
        .as_str()
        .filter(|value| !value.trim().is_empty());
    let detail: Vec<&str> = sublabel.into_iter().chain([context.as_str()]).collect();
    let aria = locale.text(
        "node.focus.detail",
        &[("label", &node.label), ("detail", &detail.join(", "))],
    );
    let title: Vec<&str> = [node.label.as_str()].into_iter().chain(detail).collect();
    write!(out, "<g id=\"node-{0}\" data-node-id=\"{0}\" data-node-label=\"{1}\" tabindex=\"0\" role=\"button\" aria-label=\"{2}\" aria-pressed=\"false\" data-node-kind=\"{tone}\"",
        escape_text(id), escape_text(&node.label), escape_text(&aria)).unwrap();
    if let Some(sublabel) = sublabel {
        write!(out, " data-node-sublabel=\"{}\"", escape_text(sublabel)).unwrap();
    }
    write!(
        out,
        " data-node-context=\"{}\" data-source-path=\"{}\" data-tree-depth=\"{depth}\"{} class=\"{}\"><title>{}</title>",
        escape_text(&context),
        escape_text(&node.authored.source_path),
        tree.attributes(id),
        tree.hue_class(id),
        escape_text(&title.join(" · "))
    )
    .unwrap();
    let b = node.bounds;
    let (x, y, w, h) = (number(b.x), number(b.y), number(b.width), number(b.height));
    // A collapsed branch shows a stacked card behind it, so the hidden
    // subtree stays visible as "there is more here".
    if !leaf {
        write!(out, "<rect data-tree-stack=\"\" x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"10\" class=\"tree-stack-{tone}\" stroke-width=\"1.2\"/>", number(b.x + 5.0), number(b.y + 5.0)).unwrap();
    }
    write!(out, "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"10\" class=\"c-mask\"/><rect data-graph-role=\"node-body\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"10\" class=\"c-{tone} {role}\" data-animate=\"node\" style=\"--step:{}\" stroke-width=\"{}\"/>",
        step(depth), if is_root { "0" } else if leaf { "1" } else { "1.2" }).unwrap();
    let stacked = node
        .authored
        .geometry_provenance
        .iter()
        .any(|entry| entry == STACKED);
    // A listed entry carries its subtree hue as a spine; every other node
    // shows it as a socket where its link lands.
    if leaf && stacked {
        write!(out, "<path class=\"tree-accent\" d=\"M {} {} V {}\" stroke-width=\"3\" stroke-linecap=\"round\" fill=\"none\"/>", number(b.x + 4.5), number(b.y + 9.0), number(b.y + b.height - 9.0)).unwrap();
    }
    if !is_root {
        let (cx, cy) = if stacked || direction(scene) != "down" {
            (b.x, b.y + b.height / 2.0)
        } else {
            (b.x + b.width / 2.0, b.y)
        };
        write!(
            out,
            "<circle cx=\"{}\" cy=\"{}\" r=\"2.6\" class=\"tree-socket\" stroke-width=\"1.4\"/>",
            number(cx),
            number(cy)
        )
        .unwrap();
    }
    let lines = |suffix: &str| -> Vec<&Text> {
        node.text
            .iter()
            .filter(|text| text.source_path.ends_with(suffix))
            .collect()
    };
    write_text(
        out,
        &lines("/label"),
        &format!("data-node-label=\"\" class=\"t-primary tree-label\" font-weight=\"{weight}\""),
    );
    write_text(
        out,
        &lines("/sublabel"),
        "data-detail=\"context\" class=\"t-muted tree-sublabel\"",
    );
    out.push_str("</g>");
}

fn body(scene: &CompiledDiagram, out: &mut String) {
    let tree = Hierarchy::of(scene);
    let locale = crate::i18n::resolve(&scene.meta);
    for (index, relation) in scene.relations.iter().enumerate() {
        // Links out of the root share one trunk and stay neutral; inside a
        // subtree every link shares its branch's hue.
        let hued = relation.from != tree.root;
        let points = relation
            .points
            .iter()
            .map(|point| format!("{},{}", number(point.x), number(point.y)))
            .collect::<Vec<_>>()
            .join(";");
        write!(out, "<path data-edge-from=\"{}\" data-edge-to=\"{}\" data-edge-key=\"{index}\"{} data-graph-role=\"edge\" data-composition-points=\"{points}\" d=\"{}\" class=\"a-default tree-edge{}\" data-animate=\"edge\" style=\"--step:{}\" stroke-width=\"{}\"/>",
            escape_text(&relation.from), escape_text(&relation.to), tree.attributes(&relation.to), rounded_path(&relation.points, 10.0),
            if hued { format!(" tree-edge-hued {}", tree.hue_class(&relation.to)) } else { " tree-edge-trunk".into() },
            step(tree.ancestors(&relation.to).len()), if hued { "1.5" } else { "1.8" }).unwrap();
    }
    for node in &scene.nodes {
        write_node(scene, &tree, &locale, node, out);
    }
    // The toggle is a separate control after its node, so a keyboard reader
    // meets "node, then its branch control" and each has one role.
    for toggle in scene
        .containers
        .iter()
        .filter(|container| container.authored.kind == "tree-toggle")
    {
        let id = toggle.authored.id.as_str();
        let label = scene
            .nodes
            .iter()
            .find(|node| node.authored.id == id)
            .map_or("", |node| node.label.as_str());
        let count = toggle.authored.facts["count"]
            .as_u64()
            .unwrap_or_default()
            .to_string();
        let params = [("label", label), ("count", count.as_str())];
        let collapse = escape_text(&locale.text("tree.toggle.collapse", &params));
        let expand = escape_text(&locale.text("tree.toggle.expand", &params));
        let (cx, cy) = (toggle.bounds.x + TOGGLE_R, toggle.bounds.y + TOGGLE_R);
        let arm = TOGGLE_R * 0.42;
        write!(out, "<g data-tree-toggle=\"{}\" data-tree-count=\"{count}\"{}{} data-tree-label-collapse=\"{collapse}\" data-tree-label-expand=\"{expand}\" role=\"button\" tabindex=\"0\" aria-expanded=\"true\" aria-label=\"{collapse}\" class=\"{}\">",
            escape_text(id), tree.attributes(id),
            if toggle.authored.facts["initiallyCollapsed"] == true { " data-tree-initially-collapsed=\"\"" } else { "" },
            tree.hue_class(id)).unwrap();
        write!(out, "<circle cx=\"{0}\" cy=\"{1}\" r=\"{2}\" class=\"tree-toggle-hit\"/><circle cx=\"{0}\" cy=\"{1}\" r=\"{3}\" class=\"tree-toggle-disc\" stroke-width=\"1.2\"/><path class=\"tree-toggle-glyph\" d=\"M {4} {1} H {5}\" stroke-width=\"1.5\" stroke-linecap=\"round\"/><path class=\"tree-toggle-glyph tree-toggle-plus\" d=\"M {0} {6} V {7}\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>",
            number(cx), number(cy), number(TOGGLE_R + 7.0), number(TOGGLE_R), number(cx - arm), number(cx + arm), number(cy - arm), number(cy + arm)).unwrap();
        if let (Some(pill), Some(text)) = (toggle.label_plate, toggle.text.first()) {
            write!(out, "<g class=\"tree-toggle-count\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"7.5\" class=\"tree-count-pill\"/><text x=\"{}\" y=\"{}\" class=\"t-primary\" font-size=\"{}\" font-weight=\"700\" text-anchor=\"middle\">{}</text></g>",
                number(pill.x), number(pill.y), number(pill.width), number(pill.height), number(text.position.x), number(text.position.y), number(text.font_size), escape_text(&text.content)).unwrap();
        }
        out.push_str("</g>");
    }
    for control in scene
        .containers
        .iter()
        .filter(|container| container.authored.kind == "tree-expand-all")
    {
        let Some(text) = control.text.first() else {
            continue;
        };
        write!(out, "<g data-tree-expand-all=\"\" role=\"button\" tabindex=\"0\" aria-label=\"{0}\"><rect x=\"{1}\" y=\"{2}\" width=\"{3}\" height=\"{4}\" rx=\"12\" stroke-width=\"1.2\"/><text x=\"{5}\" y=\"{6}\" class=\"t-primary\" font-size=\"{7}\" font-weight=\"650\">{0}</text></g>",
            escape_text(&text.content), number(control.bounds.x), number(control.bounds.y), number(control.bounds.width), number(control.bounds.height),
            number(text.position.x), number(text.position.y), number(text.font_size)).unwrap();
    }
}

fn legend_glyph(_scene: &CompiledDiagram, _legend: &Card, _baseline: f64, _out: &mut String) {}
