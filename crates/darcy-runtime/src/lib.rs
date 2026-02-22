pub mod mnist {
    use super::edn;
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io::Read;

    #[derive(Debug, Clone)]
    pub struct MnistData {
        pub images: Vec<Vec<f64>>,
        pub labels: Vec<Vec<f64>>,
    }

    pub fn load_edn_gz(path: String) -> MnistData {
        let resolved = resolve_path(&path);
        let file =
            File::open(&resolved).unwrap_or_else(|e| panic!("failed to open {}: {}", resolved, e));
        let mut decoder = GzDecoder::new(file);
        let mut buf = String::new();
        decoder
            .read_to_string(&mut buf)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", resolved, e));
        let node = edn::parse(&buf).unwrap_or_else(|e| panic!("edn parse error: {}", e));
        let (images, labels) =
            edn::to_mnist(node).unwrap_or_else(|e| panic!("edn shape error: {}", e));
        MnistData { images, labels }
    }

    fn resolve_path(path: &str) -> String {
        let p = std::path::Path::new(path);
        if p.is_absolute() {
            return path.to_string();
        }
        if let Ok(root) = std::env::var("DARCY_ROOT") {
            return std::path::Path::new(&root)
                .join(p)
                .to_string_lossy()
                .to_string();
        }
        path.to_string()
    }
}

mod edn {
    #[derive(Debug)]
    pub enum Node {
        Vec(Vec<Node>),
        Num(f64),
    }

    pub fn parse(input: &str) -> Result<Node, String> {
        let bytes = input.as_bytes();
        let mut stack: Vec<Vec<Node>> = Vec::new();
        let mut roots: Vec<Node> = Vec::new();
        let mut i = 0usize;

        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b'[' => {
                    stack.push(Vec::new());
                    i += 1;
                }
                b']' => {
                    let vec = stack.pop().ok_or_else(|| "unbalanced ']'".to_string())?;
                    let node = Node::Vec(vec);
                    if let Some(parent) = stack.last_mut() {
                        parent.push(node);
                    } else {
                        roots.push(node);
                    }
                    i += 1;
                }
                b',' | b' ' | b'\n' | b'\t' | b'\r' => {
                    i += 1;
                }
                b'+' | b'-' | b'.' | b'0'..=b'9' => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() {
                        match bytes[i] {
                            b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E' => i += 1,
                            _ => break,
                        }
                    }
                    let s = std::str::from_utf8(&bytes[start..i])
                        .map_err(|_| "invalid utf8 in number".to_string())?;
                    let val = s
                        .parse::<f64>()
                        .map_err(|e| format!("invalid number '{}': {}", s, e))?;
                    if let Some(parent) = stack.last_mut() {
                        parent.push(Node::Num(val));
                    } else {
                        return Err("number outside of vector".to_string());
                    }
                }
                _ => return Err(format!("unexpected char '{}' at {}", b as char, i)),
            }
        }

        if !stack.is_empty() {
            return Err("unclosed vector".to_string());
        }
        if roots.is_empty() {
            return Err("empty input".to_string());
        }
        if roots.len() == 1 {
            return Ok(roots.pop().unwrap());
        }
        Ok(Node::Vec(roots))
    }

    pub fn to_mnist(node: Node) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>), String> {
        let top = as_vec(&node, "root")?;
        if top.len() == 2 {
            if let Ok(images) = to_vec_vec_f64(&top[0]) {
                if let Ok(labels) = to_vec_vec_f64(&top[1]) {
                    return Ok((images, labels));
                }
            }
        }
        if top.is_empty() {
            return Err("expected [images labels] or samples".to_string());
        }
        let mut images = Vec::with_capacity(top.len());
        let mut labels = Vec::with_capacity(top.len());
        for sample in top {
            let pair = as_vec(sample, "sample")?;
            if pair.len() != 2 {
                return Err("expected [image label]".to_string());
            }
            let image = to_vec_f64(&pair[0])?;
            let label = to_vec_f64(&pair[1])?;
            images.push(image);
            labels.push(label);
        }
        Ok((images, labels))
    }

    fn to_vec_vec_f64(node: &Node) -> Result<Vec<Vec<f64>>, String> {
        let outer = as_vec(node, "outer")?;
        let mut out = Vec::with_capacity(outer.len());
        for item in outer {
            let inner = as_vec(item, "inner")?;
            let mut row = Vec::with_capacity(inner.len());
            for val in inner {
                match val {
                    Node::Num(n) => row.push(*n),
                    _ => return Err("expected number in vector".to_string()),
                }
            }
            out.push(row);
        }
        Ok(out)
    }

    fn to_vec_f64(node: &Node) -> Result<Vec<f64>, String> {
        let items = as_vec(node, "vec")?;
        let mut out = Vec::with_capacity(items.len());
        for val in items {
            match val {
                Node::Num(n) => out.push(*n),
                _ => return Err("expected number in vector".to_string()),
            }
        }
        Ok(out)
    }

    fn as_vec<'a>(node: &'a Node, label: &str) -> Result<&'a [Node], String> {
        match node {
            Node::Vec(v) => Ok(v.as_slice()),
            _ => Err(format!("expected vector for {}", label)),
        }
    }
}
