# Levels Module Test Documentation

## Overview

Comprehensive test suite for the Levels module covering both integration and unit tests. All tests validate the functionality of level management, student assignments, and proper authorization/scoping by school.

## Test Files

- `tests/integration_levels.rs` - 18 integration tests
- `src/modules/levels/service.rs` - 24 unit tests (in `#[cfg(test)]` module)

**Total: 42 tests**

**Note**: All unit tests follow Rust best practices by being embedded in the source file they test using `#[cfg(test)]` modules.

## Integration Tests (`integration_levels.rs`)

### Authentication & Authorization Tests

1. **test_create_level_as_admin**
   - Verifies admin users can create levels
   - Validates level creation returns 201 CREATED status
   - Checks response contains correct level data

2. **test_create_level_as_student_forbidden**
   - Ensures student users cannot create levels
   - Validates 403 FORBIDDEN status is returned

3. **test_unauthorized_access_to_levels**
   - Verifies requests without authentication token are rejected
   - Validates 401 UNAUTHORIZED status

### Level CRUD Operations

4. **test_create_duplicate_level_same_school**
   - Ensures level names are unique within a school
   - Validates 400 BAD_REQUEST on duplicate names

5. **test_create_same_level_name_different_schools**
   - Verifies different schools can have levels with same names
   - Tests proper school isolation

6. **test_get_levels_by_school**
   - Validates admins can retrieve list of levels for their school
   - Checks pagination metadata is included

7. **test_get_levels_scoped_by_school**
   - Ensures admins only see levels from their own school
   - Validates proper school-based filtering

8. **test_get_level_by_id**
   - Verifies admins can retrieve level details by ID
   - Checks student_count field is included

9. **test_get_level_from_different_school_not_found**
   - Ensures admins cannot access levels from other schools
   - Validates 404 NOT_FOUND status

10. **test_update_level**
    - Verifies admins can update level details
    - Validates name and description updates work correctly

11. **test_delete_level**
    - Ensures admins can delete levels
    - Validates 204 NO_CONTENT status
    - Confirms level is actually deleted

### Student Assignment Operations

12. **test_assign_students_to_level**
    - Verifies bulk student assignment to levels
    - Validates assigned_count reflects successful assignments
    - Checks failed_ids array is empty on success

13. **test_assign_students_with_invalid_ids**
    - Tests partial success scenario with mix of valid/invalid IDs
    - Validates assigned_count is correct
    - Ensures failed_ids contains invalid student IDs

14. **test_move_student_to_level**
    - Verifies students can be moved between levels
    - Validates 204 NO_CONTENT status
    - Uses PATCH method on correct endpoint

15. **test_remove_student_from_level**
    - Ensures students can be removed from levels
    - Validates 204 NO_CONTENT status

16. **test_get_students_in_level**
    - Verifies admins can retrieve list of students in a level
    - Validates correct number of students returned

17. **test_level_with_student_count**
    - Ensures student_count field accurately reflects assignments
    - Validates count updates after assignments

18. **test_cannot_assign_teacher_to_level**
    - Ensures only students can be assigned to levels
    - Validates teacher assignments fail silently
    - Checks failed_ids contains teacher ID

## Unit Tests (`service.rs` - tests module)

### Service: Create Level

1. **test_create_level_success**
   - Direct service call to create level
   - Validates returned level has correct data

2. **test_create_level_duplicate_name_same_school**
   - Tests unique constraint at service level
   - Validates BAD_REQUEST error status

3. **test_create_level_same_name_different_schools**
   - Confirms different schools can have same level names
   - Tests at service layer

### Service: Get Levels

4. **test_get_levels_by_school**
   - Validates service returns all levels for a school
   - Checks pagination metadata

5. **test_get_levels_filtered_by_name**
   - Tests name-based filtering
   - Validates case-insensitive search (ILIKE)

6. **test_get_levels_pagination**
   - Validates pagination parameters work correctly
   - Checks has_more flag is accurate

7. **test_get_level_by_id_success**
   - Direct service call to get level by ID
   - Validates student_count is 0 initially

8. **test_get_level_by_id_not_found**
   - Tests non-existent level ID handling
   - Validates NOT_FOUND error status

9. **test_get_level_by_id_different_school**
   - Ensures service enforces school isolation
   - Validates NOT_FOUND error

### Service: Update Level

10. **test_update_level_success**
    - Tests full update with name and description
    - Validates updated values are saved

11. **test_update_level_partial**
    - Tests partial updates (only name)
    - Ensures unchanged fields retain original values

12. **test_update_level_not_found**
    - Tests updating non-existent level
    - Validates NOT_FOUND error status

### Service: Delete Level

13. **test_delete_level_success**
    - Direct service call to delete level
    - Confirms level cannot be retrieved after deletion

14. **test_delete_level_not_found**
    - Tests deleting non-existent level
    - Validates NOT_FOUND error status

### Service: Assign Students

15. **test_assign_students_to_level_success**
    - Tests bulk assignment at service level
    - Validates assigned_count and empty failed_ids

16. **test_assign_students_with_invalid_ids**
    - Tests partial success with invalid IDs
    - Validates correct counts in response

17. **test_assign_students_to_nonexistent_level**
    - Tests assignment to non-existent level
    - Validates NOT_FOUND error status

### Service: Move Students

18. **test_move_student_to_level**
    - Tests moving student between levels
    - Validates student appears in new level

19. **test_move_student_to_null_level**
    - Tests removing student from level (set to NULL)
    - Validates operation succeeds

### Service: Get Students in Level

20. **test_get_students_in_level**
    - Validates service returns correct students
    - Checks student count

21. **test_get_students_in_nonexistent_level**
    - Tests getting students from non-existent level
    - Validates NOT_FOUND error status

### Service: Remove Student

22. **test_remove_student_from_level**
    - Direct service call to remove student
    - Confirms student is removed

23. **test_remove_nonexistent_student_from_level**
    - Tests removing non-existent student
    - Validates NOT_FOUND error status

### Service: Student Count

24. **test_level_student_count_updates**
    - Validates student_count updates dynamically
    - Tests count after add and remove operations
    - Ensures LEFT JOIN in query works correctly

## Test Coverage Summary

### Functional Coverage

- ✅ Level CRUD operations (create, read, update, delete)
- ✅ Bulk student assignment with partial success handling
- ✅ Student movement between levels
- ✅ Student removal from levels
- ✅ Level listing with pagination
- ✅ Level filtering by name
- ✅ Student listing within levels
- ✅ Dynamic student count calculation

### Security Coverage

- ✅ Role-based authorization (admin vs student)
- ✅ School isolation (admins see only their school's data)
- ✅ Cross-school access prevention
- ✅ Authentication requirement validation
- ✅ Role enforcement for student assignments

### Error Handling Coverage

- ✅ Duplicate level names within same school
- ✅ Non-existent level access
- ✅ Non-existent student assignment
- ✅ Invalid student IDs in bulk operations
- ✅ Cross-school unauthorized access
- ✅ Teacher assignment to levels (prevented)

### Data Integrity Coverage

- ✅ Unique constraint enforcement (level name per school)
- ✅ Foreign key relationships (school_id, level_id)
- ✅ Role validation (only students can be assigned)
- ✅ NULL handling for level_id (unassigned students)
- ✅ Cascade behavior on student reassignment

## Running Tests

### All Levels Tests
```bash
# Integration tests
cargo test --test integration_levels -- --test-threads=1

# Unit tests (embedded in service.rs)
cargo test --lib levels::service::tests -- --test-threads=1

# All library tests (includes levels and other modules)
cargo test --lib
```

### Run Integration Tests Only
```bash
cargo test --test integration_levels -- --test-threads=1
```

### Run Unit Tests Only
```bash
cargo test --lib levels::service::tests -- --test-threads=1
```

### Run Specific Test
```bash
cargo test test_create_level_as_admin -- --nocapture
```

## Test Database

All tests use SQLx's built-in test framework with:
- Automatic database setup per test
- Migration application before each test
- Transaction rollback after each test
- Parallel execution disabled (`--test-threads=1`) for stability

## Expected Results

```
Integration Tests: 18 passed
Unit Tests: 24 passed
Total: 42 passed, 0 failed
```

## Notes

- Tests use the `#[sqlx::test]` macro for automatic DB setup
- Each test runs in isolation with its own database state
- Integration tests use test helpers in `tests/common/mod.rs` for creating test data
- Unit tests are embedded in `service.rs` within a `#[cfg(test)]` module (Rust best practice)
- Integration tests use the full HTTP stack via Axum's test helpers
- Unit tests call service methods directly without HTTP layer
- This test organization follows idiomatic Rust conventions where unit tests live alongside the code they test