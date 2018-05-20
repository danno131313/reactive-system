use std::collections::HashMap;

/// `InputCellID` is a unique identifier for an input cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputCellID(u64);
/// `ComputeCellID` is a unique identifier for a compute cell.
/// Values of type `InputCellID` and `ComputeCellID` should not be mutually assignable,
/// demonstrated by the following tests:
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input: react::ComputeCellID = r.create_input(111);
/// ```
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input = r.create_input(111);
/// let compute: react::InputCellID = r.create_compute(&[react::CellID::Input(input)], |_| 222).unwrap();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComputeCellID(u64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CallbackID();

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellID {
    Input(InputCellID),
    Compute(ComputeCellID),
}

impl CellID {
    pub fn get_id(&self) -> u64 {
        match self {
            CellID::Input(cellid) => cellid.0,
            CellID::Compute(cellid) => cellid.0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RemoveCallbackError {
    NonexistentCell,
    NonexistentCallback,
}


struct ComputeCell<T> {
    func: Box<Fn(&[T]) -> T>,
    _deps: Vec<CellID>,
}

struct InputCell<T> {
    val: T,
}

enum Cell<T> {
    Compute(ComputeCell<T>),
    Input(InputCell<T>),
}

pub struct Reactor<T> {
    id: u64,
    cells: HashMap<u64, Cell<T>>,
}

impl<T: Copy + PartialEq> Cell<T> {
    fn get_val(&self, reactor: &Reactor<T>) -> T {
        match self {
            Cell::Compute(computecell) => {
                let mut computed = Vec::new();
                for dep in computecell._deps.iter() {
                    let id = dep.get_id();
                    let cell = reactor.cells.get(&id).unwrap();
                    computed.push(cell.get_val(&reactor));
                }
                let func = &computecell.func;
                func(&computed)
            },
            Cell::Input(inputcell) => inputcell.val,
        }
    }
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl <T: Copy + PartialEq> Reactor<T> {
    pub fn new() -> Self {
        Reactor { id: 0, cells: HashMap::new() }
    }

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, initial: T) -> InputCellID {
        let id = InputCellID(self.id);
        let cell = InputCell { val: initial };
        self.id = self.id + 1;
        self.cells.insert(id.0, Cell::Input(cell));
        id
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    // You do not need to reject compute functions that expect more arguments than there are
    // dependencies (how would you check for this, anyway?).
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<'b, F: Fn(&[T]) -> T + 'static>(&mut self, dependencies: &'b [CellID], compute_func: F) -> Result<ComputeCellID, CellID> {
        let mut cellcontents = Vec::new();
        for dep in dependencies {
            let id = match dep {
                CellID::Input(inputid) => inputid.0,
                CellID::Compute(computeid) => computeid.0,
            };
            match self.cells.get(&id) {
                None => return Err(*dep),
                Some(cell) => {
                    let val = cell.get_val(&self);
                    cellcontents.push(val);
                },
            };
        }

        let id = self.id;
        self.id = self.id + 1;
        let cell: ComputeCell<T> = ComputeCell {
            func: Box::new(compute_func),
            _deps: dependencies.to_owned(),
        };

        self.cells.insert(id, Cell::Compute(cell));
        let result = ComputeCellID(id);
        Ok(result)
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, id: CellID) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, id: CellID) -> Option<T> {
        match id {
            CellID::Input(cell_id) => {
                let cell = match self.cells.get(&cell_id.0) {
                    Some(cell) => cell,
                    None => return None,
                };
                
                Some(cell.get_val(&self))
            },
            CellID::Compute(cell_id) => {
                let cell = match self.cells.get(&cell_id.0) {
                    Some(k) => k,
                    None => return None,
                };

                Some(cell.get_val(&self))
            },
        }
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    //
    // Similarly, you may wonder about `get_mut(&mut self, id: CellID) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, _id: InputCellID, new_value: T) -> bool {
        match self.cells.get(&_id.0) {
            None => return false,
            Some(cell) => {
                if let Cell::Compute(_) = cell { return false; };
            },
        };

        let new_cell = InputCell { val: new_value };

        self.cells.insert(_id.0, Cell::Input(new_cell)).unwrap();
        return true;
    }

    // Adds a callback to the specified compute cell.
    //
    // Returns the ID of the just-added callback, or None if the cell doesn't exist.
    //
    // Callbacks on input cells will not be tested.
    //
    // The semantics of callbacks (as will be tested):
    // For a single set_value call, each compute cell's callbacks should each be called:
    // * Zero times if the compute cell's value did not change as a result of the set_value call.
    // * Exactly once if the compute cell's value changed as a result of the set_value call.
    //   The value passed to the callback should be the final value of the compute cell after the
    //   set_value call.
    pub fn add_callback<F: FnMut(T) -> ()>(&mut self, _id: ComputeCellID, _callback: F) -> Option<CallbackID> {
        unimplemented!()
    }

    // Removes the specified callback, using an ID returned from add_callback.
    //
    // Returns an Err if either the cell or callback does not exist.
    //
    // A removed callback should no longer be called.
    pub fn remove_callback(&mut self, cell: ComputeCellID, callback: CallbackID) -> Result<(), RemoveCallbackError> {
        unimplemented!(
            "Remove the callback identified by the CallbackID {:?} from the cell {:?}",
            callback,
            cell,
        )
    }
}
