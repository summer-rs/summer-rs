use std::{any::Any, ops::Deref, sync::Arc};

/// Component's dyn trait reference
#[derive(Debug, Clone)]
pub struct DynComponentRef(Arc<dyn Any + Send + Sync>);

impl DynComponentRef {
    /// constructor
    pub fn new<T>(component: T) -> Self
    where
        T: Any + Send + Sync,
    {
        Self(Arc::new(component))
    }

    /// Downcast to the specified type
    pub fn downcast<T>(self) -> Option<ComponentRef<T>>
    where
        T: Any + Send + Sync,
    {
        self.0.downcast::<T>().ok().map(ComponentRef::new)
    }
}

/// A component reference of a specified type
#[derive(Debug, Clone)]
pub struct ComponentRef<T>(Arc<T>);

impl<T> ComponentRef<T> {
    fn new(target_ref: Arc<T>) -> Self {
        Self(target_ref)
    }

    /// Get the raw pointer of the component
    #[inline]
    pub fn into_raw(self) -> *const T {
        Arc::into_raw(self.0)
    }
}

impl<T> Deref for ComponentRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestComponent {
        value: i32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct AnotherComponent {
        name: String,
    }

    #[test]
    fn test_dyn_component_ref_new() {
        let component = TestComponent { value: 42 };
        let dyn_ref = DynComponentRef::new(component.clone());
        
        // Should be able to downcast to the correct type
        let component_ref = dyn_ref.downcast::<TestComponent>();
        assert!(component_ref.is_some());
    }

    #[test]
    fn test_dyn_component_ref_downcast_success() {
        let component = TestComponent { value: 100 };
        let dyn_ref = DynComponentRef::new(component.clone());
        
        let downcasted = dyn_ref.downcast::<TestComponent>();
        assert!(downcasted.is_some());
        
        let component_ref = downcasted.unwrap();
        assert_eq!(component_ref.value, 100);
    }

    #[test]
    fn test_dyn_component_ref_downcast_failure() {
        let component = TestComponent { value: 42 };
        let dyn_ref = DynComponentRef::new(component);
        
        // Should fail to downcast to wrong type
        let wrong_type = dyn_ref.downcast::<AnotherComponent>();
        assert!(wrong_type.is_none());
    }

    #[test]
    fn test_component_ref_deref() {
        let component = TestComponent { value: 999 };
        let arc_component = Arc::new(component);
        let component_ref = ComponentRef::new(arc_component);
        
        // Test deref
        assert_eq!(component_ref.value, 999);
    }

    #[test]
    fn test_component_ref_clone() {
        let component = TestComponent { value: 50 };
        let dyn_ref = DynComponentRef::new(component);
        let component_ref = dyn_ref.downcast::<TestComponent>().unwrap();
        
        // Clone should work
        let cloned = component_ref.clone();
        assert_eq!(cloned.value, 50);
        assert_eq!(component_ref.value, cloned.value);
    }

    #[test]
    fn test_dyn_component_ref_clone() {
        let component = AnotherComponent {
            name: "test".to_string(),
        };
        let dyn_ref = DynComponentRef::new(component);
        let cloned_dyn_ref = dyn_ref.clone();
        
        // Both should be able to downcast
        let ref1 = cloned_dyn_ref.downcast::<AnotherComponent>().unwrap();
        assert_eq!(ref1.name, "test");
    }

    #[test]
    fn test_component_ref_into_raw() {
        let component = TestComponent { value: 123 };
        let arc_component = Arc::new(component);
        let component_ref = ComponentRef::new(arc_component.clone());
        
        let raw_ptr = component_ref.into_raw();
        
        // Convert back safely (must manually manage memory)
        unsafe {
            let recovered = Arc::from_raw(raw_ptr);
            assert_eq!(recovered.value, 123);
            // Prevent double-free by forgetting
            std::mem::forget(recovered);
            // Clean up
            drop(Arc::from_raw(raw_ptr));
        }
    }

    #[test]
    fn test_multiple_component_types() {
        let test_comp = TestComponent { value: 1 };
        let another_comp = AnotherComponent {
            name: "multi".to_string(),
        };
        
        let dyn_ref1 = DynComponentRef::new(test_comp);
        let dyn_ref2 = DynComponentRef::new(another_comp);
        
        // Each should downcast to its own type only
        assert!(dyn_ref1.clone().downcast::<TestComponent>().is_some());
        assert!(dyn_ref1.downcast::<AnotherComponent>().is_none());
        
        assert!(dyn_ref2.clone().downcast::<AnotherComponent>().is_some());
        assert!(dyn_ref2.downcast::<TestComponent>().is_none());
    }
}
