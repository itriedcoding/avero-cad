//! Statistics service.

use super::AppContext;
use crate::error::AppResult;
use crate::models::StatsOverviewView;
use crate::permissions::{Access, Permission};

pub fn overview(ctx: &AppContext, access: &Access, days: i64) -> AppResult<StatsOverviewView> {
    access.require(Permission::StatsView)?;
    ctx.db.stats_overview(days.clamp(1, 365))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tests::test_context;

    #[test]
    fn owner_can_read_statistics() {
        let (ctx, _guard) = test_context();
        let owner = ctx.db.user_by_username("owner").unwrap().unwrap();
        let access = ctx.db.load_access(owner.id).unwrap().unwrap();
        let overview = overview(&ctx, &access, 30).unwrap();
        assert_eq!(overview.range_days, 30);
        assert_eq!(overview.daily.len(), 30);
    }

    #[test]
    fn statistics_need_the_permission() {
        let (ctx, _guard) = test_context();
        let mut access = ctx
            .db
            .load_access(ctx.db.user_by_username("owner").unwrap().unwrap().id)
            .unwrap()
            .unwrap();
        access.role_key = "moderator".into();
        access.permissions.clear();
        assert_eq!(
            overview(&ctx, &access, 7).unwrap_err().code(),
            "permission_denied"
        );
    }
}
