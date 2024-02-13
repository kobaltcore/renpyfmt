use derivative::Derivative;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Debug, Clone, Default)]
pub struct RawBlock {
    pub loc: (PathBuf, usize),
    pub statements: Vec<Option<AtlStatement>>,
    pub animation: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RawRepeat {
    pub loc: (PathBuf, usize),
    pub repeats: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RawContainsExpr {
    pub loc: (PathBuf, usize),
    pub expr: String,
}

#[derive(Debug, Clone, Default)]
pub struct RawChild {
    pub loc: (PathBuf, usize),
    pub child: RawBlock,
}

#[derive(Debug, Clone, Default)]
pub struct RawParallel {
    pub loc: (PathBuf, usize),
    pub block: RawBlock,
}

#[derive(Debug, Clone, Default)]
pub struct RawChoice {
    pub loc: (PathBuf, usize),
    pub chance: String,
    pub block: RawBlock,
}

#[derive(Debug, Clone, Default)]
pub struct RawOn {
    pub loc: (PathBuf, usize),
    pub names: Vec<String>,
    pub block: RawBlock,
}

#[derive(Debug, Clone, Default)]
pub struct RawTime {
    pub loc: (PathBuf, usize),
    pub time: String,
}

#[derive(Debug, Clone, Default)]
pub struct RawFunction {
    pub loc: (PathBuf, usize),
    pub expr: String,
}

#[derive(Debug, Clone, Default)]
pub struct RawEvent {
    pub loc: (PathBuf, usize),
    pub name: String,
}

#[derive(Clone, Default, Derivative)]
#[derivative(Debug)]
pub struct RawMultipurpose {
    pub loc: (PathBuf, usize),
    pub warper: Option<String>,
    warp_function: Option<String>,
    pub duration: Option<String>,
    pub properties: Vec<(String, String)>,
    pub expressions: Vec<(String, Option<String>)>,
    splines: Vec<(String, Vec<String>)>,
    revolution: Option<String>,
    circles: Option<String>,
    #[derivative(Debug = "ignore")]
    incompatible_props: HashMap<String, Vec<String>>,
    #[derivative(Debug = "ignore")]
    compatible_pairs: Vec<Vec<String>>,
}

impl RawMultipurpose {
    pub fn new(loc: (PathBuf, usize)) -> Self {
        let incompatible_props: HashMap<String, Vec<String>> = HashMap::from([
            (
                "alignaround".into(),
                vec![
                    "xaround".into(),
                    "yaround".into(),
                    "xanchoraround".into(),
                    "yanchoraround".into(),
                ],
            ),
            (
                "align".into(),
                vec![
                    "xanchor".into(),
                    "yanchor".into(),
                    "xpos".into(),
                    "ypos".into(),
                ],
            ),
            ("anchor".into(), vec!["xanchor".into(), "yanchor".into()]),
            ("angle".into(), vec!["xpos".into(), "ypos".into()]),
            ("anchorangle".into(), vec!["xangle".into(), "yangle".into()]),
            (
                "around".into(),
                vec![
                    "xaround".into(),
                    "yaround".into(),
                    "xanchoraround".into(),
                    "yanchoraround".into(),
                ],
            ),
            ("offset".into(), vec!["xoffset".into(), "yoffset".into()]),
            ("pos".into(), vec!["xpos".into(), "ypos".into()]),
            ("radius".into(), vec!["xpos".into(), "ypos".into()]),
            (
                "anchorradius".into(),
                vec!["xanchor".into(), "yanchor".into()],
            ),
            ("size".into(), vec!["xsize".into(), "ysize".into()]),
            ("xalign".into(), vec!["xpos".into(), "xanchor".into()]),
            ("xcenter".into(), vec!["xpos".into(), "xanchor".into()]),
            (
                "xycenter".into(),
                vec![
                    "xpos".into(),
                    "ypos".into(),
                    "xanchor".into(),
                    "yanchor".into(),
                ],
            ),
            ("xysize".into(), vec!["xsize".into(), "ysize".into()]),
            ("yalign".into(), vec!["ypos".into(), "yanchor".into()]),
            ("ycenter".into(), vec!["ypos".into(), "yanchor".into()]),
        ]);

        let compatible_pairs = vec![
            vec!["radius".into(), "angle".into()],
            vec!["anchorradius".into(), "anchorangle".into()],
        ];

        Self {
            loc,
            warper: None,
            duration: None,
            warp_function: None,
            revolution: None,
            circles: None,
            splines: Vec::new(),
            properties: Vec::new(),
            expressions: Vec::new(),
            incompatible_props,
            compatible_pairs,
        }
    }

    pub fn add_warper(
        &mut self,
        name: Option<String>,
        duration: Option<String>,
        warp_function: Option<String>,
    ) {
        self.warper = name;
        self.duration = duration;
        self.warp_function = warp_function;
    }

    pub fn add_revolution(&mut self, revolution: String) {
        self.revolution = Some(revolution);
    }

    pub fn add_circles(&mut self, circles: String) {
        self.circles = Some(circles);
    }

    pub fn add_spline(&mut self, name: String, exprs: Vec<String>) {
        self.splines.push((name, exprs));
    }

    pub fn add_property(&mut self, name: String, exprs: String) -> Option<String> {
        let mut newly_set = self.incompatible_props.get(&name);

        let extra = &vec![name.clone()];
        if newly_set.is_none() {
            newly_set = Some(extra);
        }

        let newly_set = newly_set.unwrap();

        let mut old_prop = None;
        for (old, _) in &self.properties {
            let extra = &vec![old.clone()];
            let iprops = self.incompatible_props.get(old).unwrap_or(extra);
            if newly_set.iter().all(|x| iprops.contains(x)) {
                old_prop = Some(old.clone());
            }
        }

        self.properties.push((name.clone(), exprs));

        if old_prop.is_some() {
            let pair = HashSet::from([old_prop.clone().unwrap(), name]);

            for i in &self.compatible_pairs {
                if i.iter().all(|x| pair.contains(x)) {
                    old_prop = None;
                }
            }
        }

        old_prop
    }

    pub fn add_expression(&mut self, expr: String, with_clause: Option<String>) {
        self.expressions.push((expr, with_clause));
    }
}

#[derive(Debug, Clone)]
pub enum AtlStatement {
    RawRepeat(RawRepeat),
    RawBlock(RawBlock),
    RawContainsExpr(RawContainsExpr),
    RawChild(RawChild),
    RawParallel(RawParallel),
    RawChoice(RawChoice),
    RawOn(RawOn),
    RawTime(RawTime),
    RawFunction(RawFunction),
    RawEvent(RawEvent),
    RawMultipurpose(RawMultipurpose),
}
