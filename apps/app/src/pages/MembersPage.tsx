import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  Button,
  Card,
  EmptyState,
  FormField,
  ListRow,
  ListSurface,
  LoadingState,
  PermissionState,
  Status,
} from "@school-collect/ui";
import {
  createInvitation,
  listInvitations,
  listMembers,
  revokeInvitation,
  type Invitation,
  type Member,
} from "../api";
import {
  canManage,
  messageOf,
  RequestFailure,
  roleOf,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

/// Admin rights are never handed out by invitation, so the list matches the
/// roles the server accepts.
const inviteRoles = ["contributor", "coordinator", "viewer"] as const;

function expiresLabel(invitation: Invitation): string {
  const value = new Date(invitation.expiresAt);
  if (Number.isNaN(value.getTime())) {
    return "";
  }
  return `만료 ${value.toLocaleDateString("ko-KR")}`;
}

export function MembersPage({
  role,
  tenantId,
  token,
}: {
  role: string;
  tenantId: string;
  token: string;
}) {
  const online = useOnline();
  const manage = canManage(role);
  const [members, setMembers] = useState<Member[] | null>(null);
  const [invitations, setInvitations] = useState<Invitation[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [email, setEmail] = useState("");
  const [inviteRole, setInviteRole] = useState<string>("contributor");
  const [busy, setBusy] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [issued, setIssued] = useState<{ email: string; code: string } | null>(
    null,
  );

  const load = useCallback(async () => {
    try {
      const [memberList, invitationList] = await Promise.all([
        listMembers(token, tenantId),
        listInvitations(token, tenantId),
      ]);
      setMembers(memberList.members);
      setInvitations(invitationList.invitations);
      setError(null);
    } catch (caught) {
      setError(caught);
    }
  }, [tenantId, token]);

  useEffect(() => {
    if (manage) {
      void load();
    }
  }, [load, manage]);

  if (!manage) {
    return (
      <div className="app-page">
        <PermissionState
          action={
            <Button
              onClick={() => navigate({ page: "overview" })}
              variant="secondary"
            >
              홈으로
            </Button>
          }
          description="구성원 목록은 관리자와 담당자 화면입니다. 최종 권한은 서버가 판단합니다."
          title="이 화면을 볼 수 없습니다"
        />
      </div>
    );
  }

  if (error && members === null) {
    return (
      <div className="app-page">
        <RequestFailure
          action={
            <Button onClick={() => void load()} variant="secondary">
              다시 시도
            </Button>
          }
          error={error}
          online={online}
          title="구성원을 불러오지 못했습니다"
        />
      </div>
    );
  }

  async function invite(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setFormError(null);
    try {
      const created = await createInvitation(
        token,
        tenantId,
        email.trim(),
        inviteRole,
      );
      setIssued({ email: created.invitation.email, code: created.code });
      setEmail("");
      await load();
    } catch (caught) {
      setFormError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  async function revoke(invitation: Invitation) {
    setBusy(true);
    setFormError(null);
    try {
      await revokeInvitation(token, tenantId, invitation.id);
      setIssued(null);
      await load();
    } catch (caught) {
      setFormError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-page">
      <Card className="app-create">
        <div>
          <p className="app-card-eyebrow">구성원 초대</p>
          <h2>초대 코드 만들기</h2>
          <p className="app-card-description">
            초대 코드를 만들어 초대할 사람에게 직접 전달합니다. 코드는 지금 한
            번만 보이고 14일 뒤에 만료됩니다. 초대받은 사람은 로그인 후 설정
            화면의 "초대 코드로 합류"에서 코드를 입력합니다.
          </p>
        </div>
        <form className="app-form" onSubmit={invite}>
          <FormField htmlFor="invite-email" label="이메일" required>
            <input
              autoComplete="off"
              id="invite-email"
              onChange={(event) => setEmail(event.target.value)}
              placeholder="teacher@example.school.kr"
              required
              type="email"
              value={email}
            />
          </FormField>
          <FormField htmlFor="invite-role" label="역할">
            <select
              id="invite-role"
              onChange={(event) => setInviteRole(event.target.value)}
              value={inviteRole}
            >
              {inviteRoles.map((value) => (
                <option key={value} value={value}>
                  {roleOf(value)}
                </option>
              ))}
            </select>
          </FormField>
          {formError ? (
            <p className="app-form__error" role="alert">
              {formError}
            </p>
          ) : null}
          <div className="app-form__actions">
            <Button
              disabled={busy || !email.trim()}
              loading={busy}
              type="submit"
            >
              초대 만들기
            </Button>
          </div>
        </form>
        {issued ? (
          <div className="app-form">
            <p className="app-form__notice">
              {issued.email} 초대 코드가 만들어졌습니다. 이 코드는 다시 표시되지
              않습니다.
            </p>
            <FormField
              hint="코드를 복사해 초대할 사람에게 안전한 방법으로 전달하세요."
              htmlFor="invite-code"
              label="초대 코드"
            >
              <input id="invite-code" readOnly value={issued.code} />
            </FormField>
          </div>
        ) : null}
      </Card>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>초대 현황</h2>
          <Button onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>
        {invitations === null ? (
          <LoadingState
            description="초대 목록을 불러오고 있습니다."
            title="불러오는 중"
          />
        ) : invitations.length === 0 ? (
          <EmptyState
            description="위에서 첫 구성원을 초대하면 여기에 상태가 표시됩니다."
            title="초대 기록이 없습니다"
          />
        ) : (
          <ListSurface>
            {invitations.map((invitation) => (
              <ListRow
                action={
                  invitation.status === "pending" ? (
                    <Button
                      aria-label={`${invitation.email} 초대 취소`}
                      disabled={busy}
                      onClick={() => void revoke(invitation)}
                      size="small"
                      variant="quiet"
                    >
                      취소
                    </Button>
                  ) : undefined
                }
                description={`${roleOf(invitation.role)} · ${expiresLabel(invitation)}`}
                key={invitation.id}
                status={
                  <Status tone={toneOf(invitation.status)}>
                    {invitation.status === "pending"
                      ? "대기"
                      : invitation.status === "accepted"
                        ? "합류"
                        : invitation.status === "revoked"
                          ? "취소"
                          : "만료"}
                  </Status>
                }
                title={invitation.email}
              />
            ))}
          </ListSurface>
        )}
      </section>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>구성원과 역할</h2>
        </div>
        {members === null ? (
          <LoadingState
            description="구성원 목록을 불러오고 있습니다."
            title="불러오는 중"
          />
        ) : members.length === 0 ? (
          <EmptyState
            description="이 학교에 표시할 구성원이 없습니다."
            title="구성원이 없습니다"
          />
        ) : (
          <ListSurface>
            {members.map((member) => (
              <ListRow
                description={member.userId}
                key={member.userId}
                status={
                  <Status tone={toneOf(member.role)}>
                    {roleOf(member.role)}
                  </Status>
                }
                title={member.displayName ?? member.userId}
              />
            ))}
          </ListSurface>
        )}
      </section>
    </div>
  );
}
