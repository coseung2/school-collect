import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
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
  Tabs,
} from "@school-collect/ui";
import {
  createCollect,
  listCollects,
  listMembers,
  type Collect,
  type CollectItemInput,
  type Member,
} from "../api";
import {
  canManage,
  canSubmit,
  formatDue,
  isUrgentDue,
  itemKeyFromLabel,
  labelOf,
  messageOf,
  RequestFailure,
  roleOf,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

type DraftItem = {
  localId: string;
  label: string;
  required: boolean;
};

const FILTERS = [
  { id: "all", label: "전체" },
  { id: "draft", label: "작성 중" },
  { id: "published", label: "진행 중" },
  { id: "closed", label: "마감" },
];

function newItem(): DraftItem {
  return {
    localId:
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `item-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    label: "",
    required: true,
  };
}

export function CollectsPage({
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
  const [collects, setCollects] = useState<Collect[] | null>(null);
  const [members, setMembers] = useState<Member[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [filter, setFilter] = useState("all");
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [dueLocal, setDueLocal] = useState("");
  const [items, setItems] = useState<DraftItem[]>([newItem()]);
  const [selected, setSelected] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [list, roster] = await Promise.all([
        listCollects(token, tenantId),
        manage ? listMembers(token, tenantId) : Promise.resolve({ members: [] }),
      ]);
      setCollects(list.collects);
      setMembers(roster.members);
      setSelected((current) => {
        if (current.length > 0) {
          return current;
        }
        return roster.members
          .filter((member) => canSubmit(member.role))
          .map((member) => member.userId);
      });
      setError(null);
    } catch (caught) {
      setError(caught);
    }
  }, [manage, tenantId, token]);

  useEffect(() => {
    void load();
  }, [load]);

  const visible = useMemo(() => {
    const rows = collects ?? [];
    if (filter === "all") {
      return rows;
    }
    return rows.filter((item) => item.status === filter);
  }, [collects, filter]);

  if (!manage) {
    return (
      <div className="app-page">
        <PermissionState
          action={
            <Button
              onClick={() => navigate({ page: "assignments" })}
              variant="secondary"
            >
              내 제출로 이동
            </Button>
          }
          description="자료수합 관리 화면은 관리자와 담당자에게만 열려 있습니다. 최종 권한은 서버가 판단합니다."
          title="이 화면을 볼 수 없습니다"
        />
      </div>
    );
  }

  if (error && collects === null) {
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
          title="수합 목록을 불러오지 못했습니다"
        />
      </div>
    );
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    const used = new Set<string>();
    const payloadItems: CollectItemInput[] = items
      .map((item) => ({ ...item, label: item.label.trim() }))
      .filter((item) => item.label.length > 0)
      .map((item) => ({
        key: itemKeyFromLabel(item.label, used),
        label: item.label,
        required: item.required,
      }));

    setBusy(true);
    setFormError(null);
    try {
      const created = await createCollect(
        token,
        tenantId,
        title.trim(),
        description.trim(),
        payloadItems,
        selected,
        dueLocal ? new Date(dueLocal).toISOString() : null,
      );
      setTitle("");
      setDescription("");
      setDueLocal("");
      setItems([newItem()]);
      navigate({ page: "collect", id: created.id });
    } catch (caught) {
      setFormError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-page">
      <Card className="app-create">
        <h2>새 수합 만들기</h2>
        <form className="app-form" onSubmit={submit}>
          <FormField htmlFor="collect-title" label="제목" required>
            <input
              id="collect-title"
              onChange={(event) => setTitle(event.target.value)}
              required
              value={title}
            />
          </FormField>
          <FormField htmlFor="collect-description" label="설명">
            <textarea
              id="collect-description"
              onChange={(event) => setDescription(event.target.value)}
              rows={3}
              value={description}
            />
          </FormField>
          <FormField
            hint="지정하지 않으면 마감일 없이 배포됩니다."
            htmlFor="collect-due"
            label="마감일"
          >
            <input
              id="collect-due"
              onChange={(event) => setDueLocal(event.target.value)}
              type="datetime-local"
              value={dueLocal}
            />
          </FormField>

          <div className="app-stack">
            <div className="app-section-heading">
              <h3>항목</h3>
              <Button
                onClick={() => setItems((current) => [...current, newItem()])}
                size="small"
                type="button"
                variant="quiet"
              >
                항목 추가
              </Button>
            </div>
            {items.length === 0 ? (
              <p className="app-card-description">
                항목이 없어도 초안은 만들 수 있습니다. 배포하려면 항목이 필요합니다.
              </p>
            ) : (
              <ul className="app-item-editor">
                {items.map((item, index) => (
                  <li className="app-item-editor__row" key={item.localId}>
                    <FormField
                      htmlFor={`collect-item-${item.localId}`}
                      label={`항목 ${index + 1}`}
                    >
                      <input
                        id={`collect-item-${item.localId}`}
                        onChange={(event) =>
                          setItems((current) =>
                            current.map((row) =>
                              row.localId === item.localId
                                ? { ...row, label: event.target.value }
                                : row,
                            ),
                          )
                        }
                        value={item.label}
                      />
                    </FormField>
                    <label className="app-check">
                      <input
                        checked={item.required}
                        onChange={(event) =>
                          setItems((current) =>
                            current.map((row) =>
                              row.localId === item.localId
                                ? { ...row, required: event.target.checked }
                                : row,
                            ),
                          )
                        }
                        type="checkbox"
                      />
                      필수
                    </label>
                    <Button
                      onClick={() =>
                        setItems((current) =>
                          current.filter((row) => row.localId !== item.localId),
                        )
                      }
                      size="small"
                      type="button"
                      variant="quiet"
                    >
                      삭제
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="app-stack">
            <div className="app-section-heading">
              <h3>대상</h3>
              <p className="app-card-description">
                기본값은 제출 가능한 구성원입니다. 모두 해제하면 서버가 기본 대상을 정합니다.
              </p>
            </div>
            {members === null ? (
              <LoadingState
                description="구성원을 불러오고 있습니다."
                title="불러오는 중"
              />
            ) : members.length === 0 ? (
              <EmptyState
                description="대상 목록이 비어 있습니다. 빈 대상으로 만들면 서버 기본값을 사용합니다."
                title="구성원이 없습니다"
              />
            ) : (
              <ul className="app-check-list">
                {members.map((member) => (
                  <li key={member.userId}>
                    <label className="app-check">
                      <input
                        checked={selected.includes(member.userId)}
                        onChange={(event) => {
                          const checked = event.target.checked;
                          setSelected((current) =>
                            checked
                              ? [...current, member.userId]
                              : current.filter((id) => id !== member.userId),
                          );
                        }}
                        type="checkbox"
                      />
                      <span>
                        {member.displayName ?? member.userId}
                        <span className="app-check__meta">
                          {roleOf(member.role)}
                        </span>
                      </span>
                    </label>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {formError ? (
            <p className="app-form__error" role="alert">
              {formError}
            </p>
          ) : null}
          <div className="app-form__actions">
            <Button loading={busy} type="submit">
              수합 만들기
            </Button>
          </div>
        </form>
      </Card>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>수합 목록</h2>
          <Button onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>
        <Tabs items={FILTERS} label="수합 상태" onChange={setFilter} value={filter} />
        {collects === null ? (
          <LoadingState
            description="수합 목록을 불러오고 있습니다."
            title="불러오는 중"
          />
        ) : visible.length === 0 ? (
          <EmptyState
            description={
              filter === "all"
                ? "위에서 첫 수합을 만들면 여기에 표시됩니다."
                : "이 상태의 수합이 없습니다."
            }
            title="표시할 수합이 없습니다"
          />
        ) : (
          <ListSurface>
            {visible.map((collect) => (
              <ListRow
                action={
                  <Button
                    onClick={() => navigate({ page: "collect", id: collect.id })}
                    size="small"
                    variant="secondary"
                  >
                    열기
                  </Button>
                }
                description={formatDue(collect.dueAt)}
                key={collect.id}
                status={
                  <Status tone={toneOf(collect.status)}>
                    {labelOf(collect.status)}
                  </Status>
                }
                title={collect.title}
                urgent={isUrgentDue(collect.dueAt, collect.status)}
              />
            ))}
          </ListSurface>
        )}
      </section>
    </div>
  );
}
