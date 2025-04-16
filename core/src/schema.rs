#[derive(Debug)]
enum SchemaType {
    Array {
        max_items: Option<usize>,
        min_items: usize,
        unique_items: bool,
    },
    Const(ConstType),
    Enum(Vec<SchemaType>),
    Map {
        max_properties: Option<usize>,
        min_properties: usize,
        required: Vec<String>,
        dependent_required: HashMap<String, Vec<String>>,
    },
    Null,
    Numeric {
        multiple_of: Option<f64>,
        maximum: Option<f64>,
        exclusive_maximum: Option<f64>,
        minimum: Option<f64>,
        exclusive_minimum: Option<f64>,
    },
    String {
        max_length: Option<usize>,
        min_length: usize,
        pattern: Option<Regex>,
    },
}


#[derive(Debug)]
enum ConstType {
    Array(Vec<ConstType>),
    Boolean(bool),
    Map(HashMap<String, ConstType>),
    Numeric(f64),
    String(String),
}
