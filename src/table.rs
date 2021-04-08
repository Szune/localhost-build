#[derive(Default, Clone)]
pub struct Cell {
    pub content: String,
    pub length: usize,
}

impl Cell {
    pub fn new(content: String) -> Cell {
        let len = content.len();
        Cell {
            content,
            length: len,
        }
    }
}

#[derive(Default, Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
}

#[derive(Default, Clone)]
pub struct Header {
    pub content: String,
    pub length: usize,
}

impl Header {
    pub fn new(content: String) -> Header {
        let len = content.len();
        Header {
            content,
            length: len,
        }
    }
}

#[derive(Default)]
pub struct Table {
    pub headers: Vec<Header>,
    pub rows: Vec<Row>,
    pub cell_margin: usize,
}

impl Table {
    pub fn new(cell_margin: usize) -> Table {
        Table {
            cell_margin,
            ..Default::default()
        }
    }

    pub fn add_row(&mut self, cells: Vec<String>) {
        self.rows.push(Row {
            cells: cells.into_iter().map(Cell::new).collect(),
        });
    }
    pub fn set_headers(&mut self, headers: Vec<String>) {
        self.headers = headers.into_iter().map(Header::new).collect();
    }

    pub fn print(&self) {
        // print formatted table
        // TODO: do this but more readable at some point
        let header_count: usize = self.headers.len();
        let header_max_sizes: Vec<usize> = self.headers.iter().cloned().map(|h| h.length).collect();

        let mut cell_max_sizes: Vec<usize> = Vec::new();

        for i in 0..header_count {
            let cell_max = self
                .rows
                .iter()
                .max_by(|x, y| x.cells[i].length.cmp(&y.cells[i].length))
                .map(|r| r.cells[i].length)
                .expect("Wrong number of cells in table row (has to equal amount of header cells)");
            cell_max_sizes.push(cell_max);
        }

        let mut max_sizes_all: Vec<usize> = Vec::new();
        for i in 0..header_count {
            max_sizes_all.push(usize::max(header_max_sizes[i], cell_max_sizes[i]));
        }

        for i in 0..header_count {
            print!(
                "{}{}",
                &self.headers[i].content,
                " ".repeat(max_sizes_all[i] - self.headers[i].length + self.cell_margin)
            );
        }
        println!();

        for i in 0..header_count {
            print!(
                "{}{}",
                "-".repeat(self.headers[i].length),
                " ".repeat(max_sizes_all[i] - self.headers[i].length + self.cell_margin)
            );
        }
        println!();

        for row in &self.rows {
            for i in 0..header_count {
                print!(
                    "{}{}",
                    row.cells[i].content,
                    " ".repeat(max_sizes_all[i] - row.cells[i].length + self.cell_margin)
                );
            }
            println!();
        }
    }
}
