// Inventario y progreso de la misión "despertar los sistemas de la nave".
use crate::math::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    CeldaEnergia,
    CableFibra,
    CristalNavegacion,
    TanqueOxigeno,
    ChipCuantico,
}

impl Material {
    pub const TODOS: [Material; 5] = [
        Material::CeldaEnergia,
        Material::CableFibra,
        Material::CristalNavegacion,
        Material::TanqueOxigeno,
        Material::ChipCuantico,
    ];
    pub fn nombre(&self) -> &'static str {
        match self {
            Material::CeldaEnergia => "Celda de energía",
            Material::CableFibra => "Cable de fibra",
            Material::CristalNavegacion => "Cristal de navegación",
            Material::TanqueOxigeno => "Tanque de oxígeno",
            Material::ChipCuantico => "Chip cuántico",
        }
    }
    pub fn color(&self) -> V3 {
        match self {
            Material::CeldaEnergia => v3(1.0, 0.85, 0.2),
            Material::CableFibra => v3(0.3, 1.0, 0.5),
            Material::CristalNavegacion => v3(0.4, 0.9, 1.0),
            Material::TanqueOxigeno => v3(0.55, 0.65, 1.0),
            Material::ChipCuantico => v3(1.0, 0.3, 0.9),
        }
    }
}

pub struct Inventario {
    pub mochila: Vec<Material>,
    pub instalados: Vec<Material>,
    pub capacidad: usize,
}

impl Inventario {
    pub fn nuevo() -> Inventario {
        Inventario {
            mochila: vec![],
            instalados: vec![],
            capacidad: 8,
        }
    }

    pub fn recoger(&mut self, m: Material) -> Result<(), &'static str> {
        if self.mochila.contains(&m) || self.instalados.contains(&m) {
            return Err("Ya tienes ese material");
        }
        if self.mochila.len() >= self.capacidad {
            return Err("Mochila llena");
        }
        self.mochila.push(m);
        Ok(())
    }

    // Instala todo lo que lleva la mochila; devuelve cuántos materiales se instalaron.
    pub fn instalar(&mut self) -> usize {
        let n = self.mochila.len();
        self.instalados.append(&mut self.mochila);
        n
    }

    pub fn faltan(&self) -> Vec<Material> {
        Material::TODOS
            .iter()
            .copied()
            .filter(|m| !self.instalados.contains(m))
            .collect()
    }
    pub fn total_recogido(&self) -> usize {
        self.mochila.len() + self.instalados.len()
    }
    pub fn sistemas_listos(&self) -> bool {
        self.instalados.len() == Material::TODOS.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recoger_instalar_y_completar() {
        let mut i = Inventario::nuevo();
        assert!(i.recoger(Material::CableFibra).is_ok());
        assert!(i.recoger(Material::CableFibra).is_err());
        assert_eq!(i.instalar(), 1);
        assert!(i.mochila.is_empty() && i.instalados.len() == 1);
        assert!(i.recoger(Material::CableFibra).is_err());
        for m in Material::TODOS {
            let _ = i.recoger(m);
        }
        i.instalar();
        assert!(i.sistemas_listos() && i.faltan().is_empty());
    }
    #[test]
    fn mochila_con_limite() {
        let mut i = Inventario {
            mochila: vec![],
            instalados: vec![],
            capacidad: 2,
        };
        assert!(i.recoger(Material::CeldaEnergia).is_ok());
        assert!(i.recoger(Material::ChipCuantico).is_ok());
        assert_eq!(i.recoger(Material::TanqueOxigeno), Err("Mochila llena"));
    }
}
