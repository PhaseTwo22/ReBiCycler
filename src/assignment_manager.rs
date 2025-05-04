use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

/// I'm tired of implementing this in several places.
/// I want a generic thing that I can use to manage my assignments and units.
/// It needs to:
/// - CRUD assignments and roles
pub struct AssignmentManager<A, R, I>
where
    A: Identity<I>,
    R: Identity<I> + Clone + Hash + Eq,
    I: Hash + Eq + Copy,
{
    assignees: HashMap<I, A>,
    roles: HashMap<I, R>,
    assignments: HashMap<I, RoleAssignment<I>>,
}

#[derive(Clone)]
pub struct RoleAssignment<I>
where
    I: Hash + Eq + Copy,
{
    assignee: I,
    assigned_role: I,
}

impl<I> RoleAssignment<I>
where
    I: Hash + Eq + Copy,
{
    const fn assignee(&self) -> I {
        self.assignee
    }
    const fn assigned_role(&self) -> I {
        self.assigned_role
    }
}

impl<A, R, I> Default for AssignmentManager<A, R, I>
where
    A: Identity<I>,
    R: Identity<I> + Clone + Hash + Eq,

    I: Hash + Eq + Copy,
{
    fn default() -> Self {
        Self {
            assignments: HashMap::new(),
            roles: HashMap::new(),
            assignees: HashMap::new(),
        }
    }
}
impl<A, R, I> Assigns<A, R, I> for AssignmentManager<A, R, I>
where
    A: Identity<I>,
    R: Identity<I> + Clone + Hash + Eq,
    I: Hash + Eq + Copy,
{
    fn assign(&mut self, assignee: A, role: R) -> Result<(), AssignmentError<A, R, I>> {
        let role = self
            .roles
            .get(&role.id())
            .ok_or(AssignmentError::RoleDoesntExist(role.id()))?;

        match self.assignments.entry(assignee.id()) {
            Entry::Vacant(e) => {
                e.insert(RoleAssignment {
                    assignee: assignee.id(),
                    assigned_role: role.id(),
                });
                Ok(())
            }
            Entry::Occupied(e) => Err(AssignmentError::AlreadyAssigned(assignee, e.get().clone())),
        }
    }

    fn unassign(&mut self, assignee_id: I) -> Result<RoleAssignment<I>, AssignmentError<A, R, I>> {
        self.assignments
            .remove(&assignee_id)
            .ok_or(AssignmentError::<A, R, I>::NotAssignedHere(assignee_id))
    }

    fn add_role(&mut self, role: R) -> Result<(), AssignmentError<A, R, I>> {
        if self.roles.contains_key(&role.id()) {
            return Err(AssignmentError::RoleAlreadyExists(role));
        }
        self.roles.insert(role.id(), role);
        Ok(())
    }

    fn remove_role(&mut self, role_id: I) -> Result<Vec<I>, AssignmentError<A, R, I>> {
        let role = self
            .roles
            .remove(&role_id)
            .ok_or(AssignmentError::RoleDoesntExist(role_id))?;

        let had_that_role: Vec<I> = self
            .assignments
            .values()
            .filter_map(|role_assignment| {
                if role_assignment.assigned_role() == role.id() {
                    Some(role_assignment.assignee())
                } else {
                    None
                }
            })
            .collect();

        for assignee_id in &had_that_role {
            self.unassign(*assignee_id);
        }

        Ok(had_that_role)
    }

    fn count_assignments(&self) -> HashMap<&R, usize> {
        let mut counts = HashMap::new();
        for r in self.roles.values() {
            counts.entry(r).or_insert(0);
        }

        for assigmnent in self.assignments.values() {
            let role_id = assigmnent.assigned_role();
            if let Some(role) = self.roles.get(&role_id) {
                counts
                    .entry(role)
                    .and_modify(|count| *count += 1)
                    .or_insert(0);
            }
        }

        counts
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a A, &'a R)>
    where
        A: 'a,
        R: 'a,
    {
        let combinator = |aid, rid| {
            if let (Some(assignee), Some(role)) = (self.assignees.get(&aid), self.roles.get(&rid)) {
                Some((assignee, role))
            } else {
                None
            }
        };
        self.assignments.values().filter_map(move |assignment| {
            combinator(assignment.assignee(), assignment.assigned_role())
        })
    }
}

pub trait Assigns<A, R, I>
where
    A: Identity<I>,
    R: Identity<I>,
    I: Eq + Hash + Copy,
{
    fn assign(&mut self, assignee: A, role: R) -> Result<(), AssignmentError<A, R, I>>;
    fn unassign(&mut self, assignee_id: I) -> Result<RoleAssignment<I>, AssignmentError<A, R, I>>;
    fn add_role(&mut self, role: R) -> Result<(), AssignmentError<A, R, I>>;
    fn remove_role(&mut self, role_id: I) -> Result<Vec<I>, AssignmentError<A, R, I>>;
    fn count_assignments(&self) -> HashMap<&R, usize>;
    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a A, &'a R)>
    where
        A: 'a,
        R: 'a;
}

pub trait Identity<I>
where
    I: Eq + Hash + Copy,
{
    fn id(&self) -> I;
}

pub trait Commands<C, A, R, I>
where
    C: Assigns<A, R, I>,
    A: Identity<I>,
    R: Identity<I> + Clone,
    I: Eq + Hash + Copy,
{
    fn update_assignments(&mut self) -> Result<(), AssignmentError<A, R, I>>;
    fn issue_commands(&self) -> Vec<(A, C)>;
}

pub enum AssignmentError<A, R, I>
where
    I: Eq + Hash + Copy,
{
    AlreadyAssigned(A, RoleAssignment<I>),
    NotAssignedHere(I),
    RoleAlreadyExists(R),
    RoleDoesntExist(I),
}
