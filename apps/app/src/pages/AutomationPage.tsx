import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  Button,
  Card,
  EmptyState,
  ErrorState,
  FormField,
  ListRow,
  ListSurface,
  LoadingState,
  Status,
} from "@school-collect/ui";
import {
  automationKindLabel,
  automationRunsInExtension,
  automationErrorMessage,
  createShortcutRecipe,
  deleteAutomationRecipe,
  describeAutomationRecipe,
  fetchAutomationBridgeInfo,
  listAutomationRecipes,
  openAutomationRecipe,
  saveAutomationRecipe,
  type AutomationRecipe,
  type AutomationBridgeInfo,
} from "../automation";
export function AutomationPage() {
  const [recipes, setRecipes] = useState<AutomationRecipe[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [targetUrl, setTargetUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [listNotice, setListNotice] = useState<string | null>(null);
  const [undoRecipe, setUndoRecipe] = useState<AutomationRecipe | null>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [bridge, setBridge] = useState<AutomationBridgeInfo | null>(null);
  const [bridgeError, setBridgeError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setRecipes(await listAutomationRecipes());
      setLoadError(null);
    } catch (caught) {
      setRecipes(null);
      setLoadError(automationErrorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    let cancelled = false;
    fetchAutomationBridgeInfo()
      .then((value) => {
        if (!cancelled) {
          setBridge(value);
          setBridgeError(null);
        }
      })
      .catch((caught: unknown) => {
        if (!cancelled) {
          setBridgeError(automationErrorMessage(caught));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function create(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const next = await saveAutomationRecipe(createShortcutRecipe(name, targetUrl));
      setRecipes(next);
      setLoadError(null);
      setName("");
      setTargetUrl("");
      setNotice("업무 버튼을 등록했습니다.");
      setPendingDeleteId(null);
      setUndoRecipe(null);
      setListNotice(null);
    } catch (caught) {
      setError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  /**
   * 삭제는 두 번 눌러 확인합니다. 확인 전에는 파일을 건드리지 않고,
   * 삭제한 뒤에는 같은 id로 되돌릴 수 있게 레시피를 보관합니다.
   */
  async function remove(recipe: AutomationRecipe) {
    if (pendingDeleteId !== recipe.id) {
      setPendingDeleteId(recipe.id);
      setListError(null);
      setListNotice(null);
      return;
    }

    setBusy(true);
    setListError(null);
    setListNotice(null);
    try {
      setRecipes(await deleteAutomationRecipe(recipe.id));
      setLoadError(null);
      setUndoRecipe(recipe);
      setListNotice("업무 버튼을 삭제했습니다.");
      setPendingDeleteId(null);
    } catch (caught) {
      setListError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function restore(recipe: AutomationRecipe) {
    setBusy(true);
    setListError(null);
    setListNotice(null);
    try {
      setRecipes(await saveAutomationRecipe(recipe));
      setLoadError(null);
      setUndoRecipe(null);
      setListNotice("업무 버튼을 되돌렸습니다.");
    } catch (caught) {
      setListError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function open(recipe: AutomationRecipe) {
    setBusy(true);
    setListError(null);
    setListNotice(null);
    setUndoRecipe(null);
    try {
      await openAutomationRecipe(recipe.id);
      setListNotice("브라우저 열기를 요청했습니다.");
    } catch (caught) {
      setListError(automationErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-page">
      <Card className="app-create">
        <div>
          <p className="app-card-eyebrow">로컬 업무 버튼</p>
          <h2>바로가기 등록</h2>
          <p className="app-card-description">
            업무포털에 이미 로그인한 기본 브라우저에서 등록한 화면을 엽니다. 쿠키,
            인증서, 비밀번호는 School Collect에 저장하지 않습니다.
          </p>
          <p className="app-card-description">
            버튼은 School Collect 계정이 아니라 이 컴퓨터의 사용자 설정에 저장됩니다.
            같은 OS 계정을 함께 쓰면 다른 사람에게도 보일 수 있으므로 공용 PC에서는
            등록하지 마세요.
          </p>
        </div>

        <form className="app-form" onSubmit={create}>
          <FormField htmlFor="automation-name" label="버튼 이름" required>
            <input
              id="automation-name"
              maxLength={80}
              onChange={(event) => setName(event.target.value)}
              placeholder="예: 기안"
              required
              value={name}
            />
          </FormField>
          <FormField
            hint="http/https 주소만 저장합니다. 로그인 토큰이 포함된 URL은 등록하지 마세요."
            htmlFor="automation-target"
            label="대상 URL"
            required
          >
            <input
              id="automation-target"
              onChange={(event) => setTargetUrl(event.target.value)}
              placeholder="https://..."
              required
              type="url"
              value={targetUrl}
            />
          </FormField>

          {error ? (
            <p className="app-form__error" role="alert">
              {error}
            </p>
          ) : null}
          {notice ? <p className="app-form__notice">{notice}</p> : null}

          <div className="app-form__actions">
            <Button disabled={!name.trim() || !targetUrl.trim()} loading={busy} type="submit">
              버튼 추가
            </Button>
          </div>
        </form>
      </Card>

      <Card>
        <div>
          <p className="app-card-eyebrow">브라우저 확장 연결</p>
          <h2>현재 화면 등록 준비</h2>
          <p className="app-card-description">
            Chrome/Edge에 이 저장소의 apps/extension 폴더를 압축해제된 확장으로
            설치한 뒤, 아래 토큰을 확장 팝업에 붙여넣으면 현재 탭의 주소와 제목을
            이 목록에 바로 등록할 수 있습니다. 브리지는 이 컴퓨터(127.0.0.1)에서만
            응답하고, 토큰이 없으면 요청을 거부합니다.
          </p>
        </div>
        {bridgeError ? (
          <p className="app-form__error" role="alert">
            {bridgeError}
          </p>
        ) : null}
        {bridge ? (
          <>
            <FormField htmlFor="bridge-port" label="포트">
              <input id="bridge-port" readOnly value={bridge.port} />
            </FormField>
            <FormField
              hint="확장 팝업의 '연결 토큰'에 붙여넣으세요. 다른 사람에게 공유하지 마세요."
              htmlFor="bridge-token"
              label="연결 토큰"
            >
              <input id="bridge-token" readOnly value={bridge.token} />
            </FormField>
            <FormField
              hint="확장이 설치된 브라우저가 이 ID와 일치해야 연결됩니다."
              htmlFor="bridge-extension-id"
              label="확장 ID"
            >
              <input id="bridge-extension-id" readOnly value={bridge.extensionId} />
            </FormField>
          </>
        ) : null}
      </Card>

      <section className="app-section">
        <div className="app-section-heading">
          <div>
            <h2>내 업무 버튼</h2>
            <p className="app-card-description">
              확장 팝업에서 현재 화면을 바로가기로 등록하거나, 화면 요소를 골라
              자동입력 레시피를 만들 수 있습니다. 자동입력 실행과 결과 검증은 확장에서
              하고, 이 목록은 저장된 레시피를 보여 줍니다.
            </p>
          </div>
          <Button disabled={loading} onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>

        {listError ? (
          <p className="app-form__error" role="alert">
            {listError}
          </p>
        ) : null}
        {listNotice ? (
          <p className="app-form__notice">
            {listNotice}
            {undoRecipe ? (
              <>
                {" "}
                <Button
                  aria-label={`${undoRecipe.name} 삭제 되돌리기`}
                  disabled={busy}
                  onClick={() => void restore(undoRecipe)}
                  size="small"
                  variant="quiet"
                >
                  실행 취소
                </Button>
              </>
            ) : null}
          </p>
        ) : null}

        {loading ? (
          <LoadingState description="로컬 자동화 설정을 불러오고 있습니다." title="불러오는 중" />
        ) : loadError ? (
          <ErrorState
            action={
              <Button onClick={() => void load()} variant="secondary">
                다시 시도
              </Button>
            }
            description={loadError}
            title="업무 버튼을 불러오지 못했습니다"
          />
        ) : !recipes || recipes.length === 0 ? (
          <EmptyState
            description="위에서 기안, 품의, 출결처럼 자주 쓰는 화면을 첫 버튼으로 등록하세요."
            title="등록한 업무 버튼이 없습니다"
          />
        ) : (
          <ListSurface>
            {recipes.map((recipe) => (
              <ListRow
                action={
                  pendingDeleteId === recipe.id ? (
                    <div className="app-row-actions">
                      <Button
                        aria-label={`${recipe.name} 삭제 확인`}
                        disabled={busy}
                        onClick={() => void remove(recipe)}
                        size="small"
                      >
                        삭제 확인
                      </Button>
                      <Button
                        aria-label={`${recipe.name} 삭제 취소`}
                        disabled={busy}
                        onClick={() => setPendingDeleteId(null)}
                        size="small"
                        variant="quiet"
                      >
                        취소
                      </Button>
                    </div>
                  ) : (
                    <div className="app-row-actions">
                      <Button
                        aria-label={
                          automationRunsInExtension(recipe.kind)
                            ? `${recipe.name} 화면 열기`
                            : `${recipe.name} 열기`
                        }
                        disabled={busy}
                        onClick={() => void open(recipe)}
                        size="small"
                        variant="secondary"
                      >
                        {automationRunsInExtension(recipe.kind) ? "화면 열기" : "열기"}
                      </Button>
                      <Button
                        aria-label={`${recipe.name} 삭제`}
                        disabled={busy}
                        onClick={() => void remove(recipe)}
                        size="small"
                        variant="quiet"
                      >
                        삭제
                      </Button>
                    </div>
                  )
                }
                description={describeAutomationRecipe(recipe)}
                key={recipe.id}
                status={<Status tone="info">{automationKindLabel(recipe.kind)}</Status>}
                title={recipe.name}
              />
            ))}
          </ListSurface>
        )}
      </section>
    </div>
  );
}
