use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

use rust_sc2::prelude::*;

struct AssignmentManager<A, R> {
    assignments: HashMap<A, R>,
}
impl<A: Hash + Eq + Clone, R: Hash + Eq + Clone> Assigns<A, R> for AssignmentManager<A, R> {
    fn assign(&mut self, assignee: A, role: R) -> Result<(), AssignmentError<A>> {
        if let Entry::Vacant(e) = self.assignments.entry(assignee.clone()) {
            e.insert(role);
            Ok(())
        } else {
            Err(AssignmentError::AlreadyAssigned::<A>(assignee))
        }
    }

    fn unassign(&mut self, assignee: &A) -> Result<R, AssignmentError<A>> {
        self.assignments
            .remove(assignee)
            .ok_or(AssignmentError::<A>::NotAssignedHere(assignee.clone()))
    }

    fn remove_role(&mut self, role: R) -> Vec<A> {
        let had_that_role: Vec<A> = self
            .assignments
            .iter()
            .filter_map(|(assignee, existing_role)| {
                if *existing_role == role {
                    Some(assignee)
                } else {
                    None
                }
            })
            .cloned()
            .collect();

        for assignee in had_that_role.iter() {
            self.unassign(assignee);
        }

        had_that_role
    }

    fn count_assignments(&self) -> HashMap<&R, usize> {
        let mut counts = HashMap::new();

        for (_, role) in self.assignments.iter() {
            counts
                .entry(role)
                .and_modify(|count| *count += 1)
                .or_insert(0);
        }

        counts
    }
}

trait Assigns<A, R>
where
    A: Hash + Eq,
    R: Hash + Eq,
{
    fn assign(&mut self, assignee: A, role: R) -> Result<(), AssignmentError<A>>;
    fn unassign(&mut self, assignee: &A) -> Result<R, AssignmentError<A>>;
    fn remove_role(&mut self, role: R) -> Vec<A>;
    fn count_assignments(&self) -> HashMap<&R, usize>;
}
trait Commands<C, A, R>
where
    C: Assigns<A, R>,
    A: Hash + Eq,
    R: Hash + Eq,
{
    fn update_assignments(&mut self) -> Result<(), AssignmentError<A>>;
    fn issue_commands(&self) -> Vec<(A, C)>;
}

enum AssignmentError<A> {
    AlreadyAssigned(A),
    NotAssignedHere(A),
}
