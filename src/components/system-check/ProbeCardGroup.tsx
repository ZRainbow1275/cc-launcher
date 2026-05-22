import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronUp } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import type { ProbeItem } from "@/lib/api/contracts";

import { ProbeItemCard } from "./ProbeItemCard";

interface ProbeCardGroupProps {
  title: string;
  items: ProbeItem[];
  onRequestFix: (item: ProbeItem) => void;
  defaultOpen?: boolean;
}

interface GroupCounts {
  green: number;
  yellow: number;
  red: number;
}

function tallyGroup(items: ProbeItem[]): GroupCounts {
  return items.reduce<GroupCounts>(
    (acc, it) => {
      if (it.status === "green") acc.green += 1;
      else if (it.status === "yellow") acc.yellow += 1;
      else if (it.status === "red" || it.status === "missing") acc.red += 1;
      return acc;
    },
    { green: 0, yellow: 0, red: 0 },
  );
}

export function ProbeCardGroup({
  title,
  items,
  onRequestFix,
  defaultOpen = true,
}: ProbeCardGroupProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(defaultOpen);
  const counts = useMemo(() => tallyGroup(items), [items]);

  return (
    <Card data-testid={`probe-group-${title}`}>
      <Collapsible open={open} onOpenChange={setOpen}>
        <CardHeader className="py-3">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-3">
              <h4 className="text-base font-semibold">{title}</h4>
              <div className="flex items-center gap-1.5 text-xs">
                <span className="rounded-full bg-emerald-100 dark:bg-emerald-900/40 text-emerald-900 dark:text-emerald-100 px-2 py-0.5 font-mono">
                  {counts.green}
                </span>
                <span className="rounded-full bg-yellow-100 dark:bg-yellow-900/40 text-yellow-900 dark:text-yellow-100 px-2 py-0.5 font-mono">
                  {counts.yellow}
                </span>
                <span className="rounded-full bg-red-100 dark:bg-red-900/40 text-red-900 dark:text-red-100 px-2 py-0.5 font-mono">
                  {counts.red}
                </span>
              </div>
            </div>
            <CollapsibleTrigger asChild>
              <Button variant="ghost" size="sm" aria-label={title}>
                {open ? (
                  <>
                    {t("systemCheck.actions.collapse")}
                    <ChevronUp className="h-4 w-4 ml-1" />
                  </>
                ) : (
                  <>
                    {t("systemCheck.actions.expand")}
                    <ChevronDown className="h-4 w-4 ml-1" />
                  </>
                )}
              </Button>
            </CollapsibleTrigger>
          </div>
        </CardHeader>
        <CollapsibleContent>
          <CardContent className="pt-0 space-y-2">
            {items.map((item) => (
              <ProbeItemCard
                key={item.id}
                item={item}
                onRequestFix={onRequestFix}
              />
            ))}
          </CardContent>
        </CollapsibleContent>
      </Collapsible>
    </Card>
  );
}
