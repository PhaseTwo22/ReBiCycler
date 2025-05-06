use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

/// I'm tired of implementing this in several places.
/// I want a generic thing that I can use to manage my assignments and units.
/// It needs to:
/// - CRUD assignments and roles
pub struct AssignmentManager<A, R, I, J>
where
    A: Identity<I>,
    R: Identity<J> + Clone + Hash + Eq,
    I: Hash + Eq + Copy,
    J: Hash + Eq + Clone,
{
    assignees: HashMap<I, A>,
    roles: HashMap<J, R>,
    assignments: HashMap<I, RoleAssignment<I, J>>,
}

#[derive(Clone)]
pub struct RoleAssignment<I, J>
where
    I: Hash + Eq + Copy,
    J: Hash + Eq + Clone,
{
    assignee: I,
    assigned_role: J,
}

impl<I, J> RoleAssignment<I, J>
where
    I: Hash + Eq + Copy,
    J: Hash + Eq + Clone,
{
    const fn assignee(&self) -> I {
        self.assignee
    }
    fn assigned_role(&self) -> J {
        self.assigned_role.clone()
    }
}

impl<A, R, I, J> Default for AssignmentManager<A, R, I, J>
where
    A: Identity<I>,
    R: Identity<J> + Clone + Hash + Eq,

    I: Hash + Eq + Copy,
    J: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self {
            assignments: HashMap::new(),
            roles: HashMap::new(),
            assignees: HashMap::new(),
        }
    }
}
impl<A, R, I, J> Assigns<A, R, I, J> for AssignmentManager<A, R, I, J>
where
    A: Identity<I>,
    R: Identity<J> + Clone + Hash + Eq,
    I: Hash + Eq + Copy,
    J: Hash + Eq + Clone,
{
    fn assign(&mut self, assignee: A, role: &R) -> Result<(), AssignmentError<A, R, I, J>> {
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

    fn get_assignee(&self, assignee_id: I) -> Result<&A, AssignmentError<A, R, I, J>> {
        self.assignees
            .get(&assignee_id)
            .ok_or(AssignmentError::NotAssignedHere(assignee_id))
    }

    fn update_assignee(
        &mut self,
        assignee_id: I,
        updated_assignee: A,
    ) -> Result<(), AssignmentError<A, R, I, J>> {
        if let std::collections::hash_map::Entry::Occupied(mut e) =
            self.assignees.entry(assignee_id)
        {
            e.insert(updated_assignee);
            Ok(())
        } else {
            Err(AssignmentError::NotAssignedHere(assignee_id))
        }
    }

    fn unassign(
        &mut self,
        assignee_id: I,
    ) -> Result<RoleAssignment<I, J>, AssignmentError<A, R, I, J>> {
        self.assignments
            .remove(&assignee_id)
            .ok_or(AssignmentError::<A, R, I, J>::NotAssignedHere(assignee_id))
    }

    fn add_role(&mut self, role: R) -> Result<(), AssignmentError<A, R, I, J>> {
        if self.roles.contains_key(&role.id()) {
            return Err(AssignmentError::RoleAlreadyExists(role));
        }
        self.roles.insert(role.id(), role);
        Ok(())
    }

    fn get_assignment(&self, assignee_id: I) -> Result<&R, AssignmentError<A, R, I, J>> {
        let assignment = self
            .assignments
            .get(&assignee_id)
            .ok_or(AssignmentError::NotAssignedHere(assignee_id))?;
        self.roles
            .get(&assignment.assigned_role)
            .ok_or(AssignmentError::RoleDoesntExist(
                assignment.assigned_role.clone(),
            ))
    }

    fn change_assignment(
        &mut self,
        assignee_id: I,
        new_role: R,
    ) -> Result<(), AssignmentError<A, R, I, J>> {
        let new_assignment = RoleAssignment {
            assignee: assignee_id,
            assigned_role: new_role.id(),
        };

        self.assignments
            .remove(&assignee_id)
            .ok_or(AssignmentError::NotAssignedHere(assignee_id))?;

        if !self.roles.contains_key(&new_role.id()) {
            return Err(AssignmentError::RoleDoesntExist(new_role.id()));
        }

        self.assignments.insert(assignee_id, new_assignment);
        Ok(())
    }

    fn remove_role(&mut self, role_id: J) -> Result<Vec<I>, AssignmentError<A, R, I, J>> {
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

    fn get_role_ids<'a>(&'a self) -> impl Iterator<Item = &'a J>
    where
        J: 'a,
    {
        self.roles.keys()
    }
}

pub trait Assigns<A, R, I, J>
where
    A: Identity<I>,
    R: Identity<J>,
    I: Eq + Hash + Copy,
    J: Hash + Eq + Clone,
{
    fn assign(&mut self, assignee: A, role: &R) -> Result<(), AssignmentError<A, R, I, J>>;
    fn get_assignee(&self, assignee_id: I) -> Result<&A, AssignmentError<A, R, I, J>>;
    fn update_assignee(
        &mut self,
        assignee_id: I,
        updated_assignee: A,
    ) -> Result<(), AssignmentError<A, R, I, J>>;
    fn unassign(
        &mut self,
        assignee_id: I,
    ) -> Result<RoleAssignment<I, J>, AssignmentError<A, R, I, J>>;

    fn add_role(&mut self, role: R) -> Result<(), AssignmentError<A, R, I, J>>;
    fn get_assignment(&self, assignee_id: I) -> Result<&R, AssignmentError<A, R, I, J>>;
    fn get_role_ids<'a>(&'a self) -> impl Iterator<Item = &'a J>
    where
        J: 'a;
    fn change_assignment(
        &mut self,
        assignee_id: I,
        new_role: R,
    ) -> Result<(), AssignmentError<A, R, I, J>>;
    fn remove_role(&mut self, role_id: J) -> Result<Vec<I>, AssignmentError<A, R, I, J>>;
    fn count_assignments(&self) -> HashMap<&R, usize>;
    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a A, &'a R)>
    where
        A: 'a,
        R: 'a;
}

pub trait Identity<I>
where
    I: Eq + Hash + Clone,
{
    fn id(&self) -> I;
}

pub trait Commands<C, A, I, D>
where
    A: Identity<I>,
    I: Eq + Hash + Copy,
{
    fn get_peon_updates(&mut self, data: &D) -> Vec<A>;
    fn apply_peon_updates(&mut self, updates: Vec<A>);
    fn issue_commands(&self) -> Vec<(I, C)>;
}

pub enum AssignmentError<A, R, I, J>
where
    I: Eq + Hash + Copy,
    J: Hash + Eq + Clone,
{
    AlreadyAssigned(A, RoleAssignment<I, J>),
    NotAssignedHere(I),
    RoleAlreadyExists(R),
    RoleDoesntExist(J),
}

pub enum CommandError {}
