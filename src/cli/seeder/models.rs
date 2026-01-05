use uuid::Uuid;

pub struct SchoolSeed {
    pub name: String,
    pub address: String,
}

pub struct LevelSeed {
    pub name: String,
    pub description: Option<String>,
    pub school_id: Uuid,
}

pub struct BranchSeed {
    pub name: String,
    pub description: Option<String>,
    pub level_id: Uuid,
}

pub struct UserSeed {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
    pub role_id: Uuid,
    pub school_id: Option<Uuid>,
    pub level_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct UsersPerSchool {
    pub admins: usize,
    pub teachers: usize,
}

impl Default for UsersPerSchool {
    fn default() -> Self {
        Self {
            admins: 2,
            teachers: 5,
        }
    }
}

#[derive(Clone)]
pub struct LevelsPerSchool {
    pub count: usize,
    pub branches_per_level: usize,
    pub students_per_branch: usize,
}

impl Default for LevelsPerSchool {
    fn default() -> Self {
        Self {
            count: 6,              // e.g., Grade 1-6
            branches_per_level: 3, // e.g., A, B, C
            students_per_branch: 25,
        }
    }
}

#[derive(Clone, Default)]
pub struct SeedConfig {
    pub num_schools: usize,
    pub users_per_school: UsersPerSchool,
    pub levels_per_school: LevelsPerSchool,
}

impl SeedConfig {
    pub fn new(num_schools: usize) -> Self {
        Self {
            num_schools,
            ..Default::default()
        }
    }

    pub fn with_users(mut self, users: UsersPerSchool) -> Self {
        self.users_per_school = users;
        self
    }

    pub fn with_levels(mut self, levels: LevelsPerSchool) -> Self {
        self.levels_per_school = levels;
        self
    }

    pub fn total_students_per_school(&self) -> usize {
        self.levels_per_school.count
            * self.levels_per_school.branches_per_level
            * self.levels_per_school.students_per_branch
    }

    pub fn total_users_per_school(&self) -> usize {
        self.users_per_school.admins
            + self.users_per_school.teachers
            + self.total_students_per_school()
    }
}
