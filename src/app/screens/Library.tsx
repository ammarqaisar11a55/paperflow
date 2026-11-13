import { Clock, SearchX, Star, Trash2 } from "lucide-react";
import { Page } from "@/components/layout/Page";
import { ActivityList } from "@/components/tools/ActivityList";
import { ToolCard } from "@/components/tools/ToolCard";
import { Button } from "@/components/ui/Button";
import { EmptyState, SectionTitle } from "@/components/ui/Display";
import { CATEGORIES, TOOLS, searchTools } from "@/features/registry";
import { useJobs } from "@/stores/jobs";
import { useNavigation } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";

export function AllTools() {
  const openTool = useNavigation((s) => s.openTool);
  const search = useNavigation((s) => s.search);
  const setSearch = useNavigation((s) => s.setSearch);
  const results = searchTools(search);

  return (
    <Page title="All tools" description={`${TOOLS.length} tools, all running locally on your computer.`}>
      {search.trim() ? (
        <>
          <SectionTitle>
            {results.length} result{results.length === 1 ? "" : "s"} for “{search.trim()}”
          </SectionTitle>
          {results.length ? (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
              {results.map((t) => (
                <ToolCard key={t.id} tool={t} onOpen={() => openTool(t.id)} />
              ))}
            </div>
          ) : (
            <EmptyState icon={SearchX} title="No matching tools" action={<Button onClick={() => setSearch("")}>Clear search</Button>}>
              Try another word, such as “merge”, “smaller” or “password”.
            </EmptyState>
          )}
        </>
      ) : (
        <div className="flex flex-col gap-9">
          {CATEGORIES.map((c) => (
            <section key={c}>
              <SectionTitle>{c}</SectionTitle>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
                {TOOLS.filter((t) => t.category === c).map((t) => (
                  <ToolCard key={t.id} tool={t} onOpen={() => openTool(t.id)} />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </Page>
  );
}

export function Recent() {
  const activity = useJobs((s) => s.activity);
  const clear = useJobs((s) => s.clearActivity);
  return (
    <Page
      title="Recent"
      description="What you processed lately. Only file names and locations are kept — never document contents."
      actions={
        activity.length > 0 && (
          <Button icon={Trash2} onClick={() => void clear()}>
            Clear history
          </Button>
        )
      }
    >
      {activity.length ? (
        <ActivityList entries={activity} />
      ) : (
        <div className="rounded-xl border border-line bg-surface">
          <EmptyState icon={Clock} title="No recent activity">
            Processed files appear here. You can turn history off in Settings.
          </EmptyState>
        </div>
      )}
    </Page>
  );
}

export function Favorites() {
  const favorites = useSettings((s) => s.settings.favorites);
  const openTool = useNavigation((s) => s.openTool);
  const navigate = useNavigation((s) => s.navigate);
  const tools = TOOLS.filter((t) => favorites.includes(t.id));
  return (
    <Page title="Favorites" description="Your most-used tools, one click away.">
      {tools.length ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
          {tools.map((t) => (
            <ToolCard key={t.id} tool={t} onOpen={() => openTool(t.id)} />
          ))}
        </div>
      ) : (
        <div className="rounded-xl border border-line bg-surface">
          <EmptyState icon={Star} title="No favorites yet" action={<Button onClick={() => navigate({ view: "tools" })}>Browse tools</Button>}>
            Hover over a tool and click the star to keep it here.
          </EmptyState>
        </div>
      )}
    </Page>
  );
}
