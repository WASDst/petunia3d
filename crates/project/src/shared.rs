//! Dados pesados compartilhados entre o documento e o histórico de Undo.
//!
//! Cada entrada de Undo guarda um `Project` inteiro. Com malhas e texturas
//! em `Shared<T>`, clonar o projeto só incrementa contadores: a cópia real
//! acontece na primeira escrita (`DerefMut` → `Arc::make_mut`) e só do dado
//! alterado. Um objeto ou textura que não mudou continua compartilhado entre
//! todos os snapshots. A serialização é transparente (mesmo layout de `T`),
//! então o formato do arquivo não muda.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `T` com cópia na escrita.
#[derive(Default)]
pub struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }

    /// Duas referências apontam para o mesmo dado (sem cópia pendente).
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
}

impl<T: Clone> Shared<T> {
    /// Devolve o valor, copiando só se ainda estiver compartilhado.
    pub fn into_inner(self) -> T {
        Arc::try_unwrap(self.0).unwrap_or_else(|shared| (*shared).clone())
    }

    /// Cópia independente do valor (equivale a `(*shared).clone()`).
    pub fn to_owned_value(&self) -> T {
        (*self.0).clone()
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}

impl<T: PartialEq> PartialEq<T> for Shared<T> {
    fn eq(&self, other: &T) -> bool {
        *self.0 == *other
    }
}

impl<T: Serialize> Serialize for Shared<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Shared<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_shares_until_the_first_write() {
        let a = Shared::new(vec![1u8, 2, 3]);
        let mut b = a.clone();
        assert!(Shared::ptr_eq(&a, &b));
        b.push(4);
        assert!(!Shared::ptr_eq(&a, &b));
        assert_eq!(*a, vec![1, 2, 3]);
        assert_eq!(*b, vec![1, 2, 3, 4]);
    }

    #[test]
    fn serialization_is_transparent() {
        let shared = Shared::new(vec![7u32, 8]);
        let plain = vec![7u32, 8];
        assert_eq!(
            serde_json::to_string(&shared).unwrap(),
            serde_json::to_string(&plain).unwrap()
        );
        let back: Shared<Vec<u32>> = serde_json::from_str("[7,8]").unwrap();
        assert_eq!(*back, plain);
    }
}
