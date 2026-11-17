//! Duty roster service.
//!
//! Every signed-in staff member can clock themselves in and out. Reading the
//! whole roster needs `shifts.view`; ending somebody else's shift needs
//! `shifts.manage`. All durations come from stored timestamps.

use super::AppContext;
use crate::db::audit::AuditEntry;
use crate::db::shifts::local_day_start;
use crate::error::AppResult;
use crate::models::{DutyShiftView, DutyStatusView};
use crate::permissions::{Access, Permission};
use crate::util::{now_ms, require_text};

const RECENT_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
const RECENT_LIMIT: usize = 25;
const HISTORY_LIMIT: usize = 200;

pub fn status(ctx: &AppContext, access: &Access) -> AppResult<DutyStatusView> {
    let current = ctx.db.current_shift(access.user_id)?;
    let can_view_roster = access.can(Permission::ShiftsView);
    let (roster, recent) = if can_view_roster {
        (
            ctx.db.on_duty_shifts()?,
            ctx.db
                .recent_shifts(local_day_start(now_ms()) - RECENT_WINDOW_MS, RECENT_LIMIT)?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(DutyStatusView {
        current,
        today_ms: ctx.db.shift_today_ms(access.user_id)?,
        week_ms: ctx.db.shift_week_ms(access.user_id)?,
        total_ms: ctx.db.shift_total_ms(access.user_id)?,
        shift_count: ctx.db.shift_count(access.user_id)?,
        roster,
        recent,
        can_view_roster,
    })
}

pub fn clock_in(ctx: &AppContext, access: &Access) -> AppResult<DutyStatusView> {
    let shift = ctx.db.open_shift(access.user_id, &access.display_name)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "shift",
        action: "shift.started",
        target: &access.display_name,
        details: &format!("Shift #{}", shift.id),
    })?;
    status(ctx, access)
}

pub fn clock_out(ctx: &AppContext, access: &Access) -> AppResult<DutyStatusView> {
    let shift = ctx
        .db
        .current_shift(access.user_id)?
        .ok_or_else(|| crate::error::AppError::conflict("You are not clocked in."))?;
    let closed = ctx.db.close_shift(shift.id, "Clocked out")?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "shift",
        action: "shift.ended",
        target: &access.display_name,
        details: &format!("{} minutes on duty", closed.duration_ms / 60_000),
    })?;
    status(ctx, access)
}

pub fn history(
    ctx: &AppContext,
    access: &Access,
    mine_only: bool,
) -> AppResult<Vec<DutyShiftView>> {
    access.require(Permission::ShiftsView)?;
    let user_id = if mine_only {
        Some(access.user_id)
    } else {
        None
    };
    ctx.db.shift_history(user_id, HISTORY_LIMIT)
}

pub fn end_shift(
    ctx: &AppContext,
    access: &Access,
    user_id: i64,
    reason: &str,
) -> AppResult<DutyShiftView> {
    access.require(Permission::ShiftsManage)?;
    let reason = require_text(reason, "Reason", 3, 200)?;
    let shift = ctx.db.close_shift_for_user(user_id, &reason)?;
    ctx.db.record_audit(&AuditEntry {
        actor_user_id: Some(access.user_id),
        actor_name: &access.display_name,
        category: "shift",
        action: "shift.ended_by_supervisor",
        target: &shift.display_name,
        details: &format!("{} minutes on duty · {reason}", shift.duration_ms / 60_000),
    })?;
    Ok(shift)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::ROLE_MODERATOR;
    use crate::services::tests::test_context;
    use std::collections::BTreeSet;

    fn moderator(permissions: &[&str]) -> Access {
        Access {
            user_id: 2,
            username: "mod".into(),
            display_name: "Mod".into(),
            role_id: 3,
            role_key: ROLE_MODERATOR.into(),
            role_name: "Moderator".into(),
            permissions: permissions
                .iter()
                .map(|value| value.to_string())
                .collect::<BTreeSet<_>>(),
        }
    }

    #[test]
    fn clock_in_and_out_round_trip_with_totals() {
        let (ctx, _guard) = test_context();
        let owner = ctx.db.user_by_username("owner").unwrap().unwrap();
        let access = ctx.db.load_access(owner.id).unwrap().unwrap();
        let before = status(&ctx, &access).unwrap();
        assert!(before.current.is_none());
        assert!(before.roster.is_empty());

        let started = clock_in(&ctx, &access).unwrap();
        assert!(started.current.is_some());
        assert_eq!(started.roster.len(), 1);
        assert!(started.can_view_roster);

        let ended = clock_out(&ctx, &access).unwrap();
        assert!(ended.current.is_none());
        assert_eq!(ended.shift_count, 1);
        assert!(ended.total_ms >= 0);
    }

    #[test]
    fn double_clock_in_is_rejected() {
        let (ctx, _guard) = test_context();
        let owner = ctx.db.user_by_username("owner").unwrap().unwrap();
        let access = ctx.db.load_access(owner.id).unwrap().unwrap();
        clock_in(&ctx, &access).unwrap();
        let err = clock_in(&ctx, &access).unwrap_err();
        assert_eq!(err.code(), "conflict");
    }

    #[test]
    fn roster_requires_the_view_permission() {
        let (ctx, _guard) = test_context();
        let owner = ctx.db.user_by_username("owner").unwrap().unwrap();
        let owner_access = ctx.db.load_access(owner.id).unwrap().unwrap();
        clock_in(&ctx, &owner_access).unwrap();

        let blind = moderator(&[]);
        let status_without_permission = status(&ctx, &blind).unwrap();
        assert!(status_without_permission.roster.is_empty());
        assert!(!status_without_permission.can_view_roster);
        assert_eq!(
            history(&ctx, &blind, false).unwrap_err().code(),
            "permission_denied"
        );
    }

    #[test]
    fn supervisors_can_end_another_users_shift() {
        let (ctx, _guard) = test_context();
        let owner = ctx.db.user_by_username("owner").unwrap().unwrap();
        let access = ctx.db.load_access(owner.id).unwrap().unwrap();
        let started = clock_in(&ctx, &access).unwrap();
        let shift_id = started.current.unwrap().id;

        let worker = moderator(&["shifts.view"]);
        assert_eq!(
            end_shift(&ctx, &worker, owner.id, "End of deployment")
                .unwrap_err()
                .code(),
            "permission_denied"
        );
        let ended = end_shift(&ctx, &access, owner.id, "End of deployment").unwrap();
        assert_eq!(ended.id, shift_id);
        assert!(!ended.on_duty);
        assert_eq!(ended.end_reason.as_deref(), Some("End of deployment"));
    }
}
