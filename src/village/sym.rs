struct Sym3D([usize; 6]);

impl Sym3D {
    const X: usize = 0;
    const NEG_X: usize = 1;
    const Y: usize = 2;
    const NEG_Y: usize = 3;
    const Z: usize = 4;
    const NEG_Z: usize = 5;
    const ID: Sym3D = Sym3D([
        Self::X,
        Self::NEG_X,
        Self::Y,
        Self::NEG_Y,
        Self::Z,
        Self::NEG_Z,
    ]);

    fn group_operation(&self, rhs: &Sym3D) -> Self {
        Self([
            rhs.0[self.0[0]],
            rhs.0[self.0[1]],
            rhs.0[self.0[2]],
            rhs.0[self.0[3]],
            rhs.0[self.0[4]],
            rhs.0[self.0[5]],
        ])
    }

    fn from_map(map: [(usize, usize); 6]) -> Self {
        let mut permuation = [0; 6];
        for (from, to) in map.into_iter() {
            permuation[to] = from;
        }
        Self(permuation)
    }
}
impl std::ops::Mul for Sym3D {
    type Output = Sym3D;
    fn mul(self, rhs: Self) -> Self::Output {
        self.group_operation(&rhs)
    }
}
impl std::ops::Mul for &Sym3D {
    type Output = Sym3D;
    fn mul(self, rhs: Self) -> Self::Output {
        self.group_operation(rhs)
    }
}
impl PartialEq for Sym3D {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn ne(&self, other: &Self) -> bool {
        self.0 != other.0
    }
}
struct SymGroup3D {
    elements: Vec<Sym3D>,
}
impl SymGroup3D {
    pub fn cycle(e: Sym3D) -> Self {
        let mut elements = vec![e];
        while elements.last() != elements.first() {
            elements.push(elements.first().unwrap() * elements.last().unwrap())
        }
        Self { elements }
    }

    pub fn product(a: Self, b: Self) -> Self {
        let mut product = Self::merge(&a, &b);
        product.expand_to_subgroup();
        return product;
    }

    fn expand_to_subgroup(&mut self) {
        loop {
            let new = self
                .elements
                .iter()
                .zip(self.elements.iter())
                .map(|(f, g)| f * g)
                .filter(|e| self.elements.contains(e));

            let length = self.elements.len();
            self.elements.extend(new);
            if self.elements.len() == length {
                break;
            }
        }
    }

    fn merge(a: &Self, b: &Self) -> Self {
        Self {
            elements: a
                .elements
                .into_iter()
                .filter(|e| !b.elements.contains(e))
                .chain(b.elements.into_iter())
                .collect::<Vec<Sym3D>>(),
        }
    }
}
