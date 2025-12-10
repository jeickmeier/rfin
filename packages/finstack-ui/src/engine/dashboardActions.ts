import { z } from "zod";

import {
  ComponentInstance,
  ComponentInstanceSchema,
  ComponentModeSchema,
  DashboardDefinition,
  DashboardDefinitionSchema,
  LayoutTemplate,
} from "../schemas/dashboard";

const PlacementSchema = z.object({
  column: z.enum(["left", "right"]).optional(),
  tabId: z.string().uuid().optional(),
  sectionId: z.string().uuid().optional(),
  index: z.number().int().nonnegative().optional(),
});

const AddComponentActionSchema = z.object({
  kind: z.literal("add_component"),
  component: ComponentInstanceSchema,
  placement: PlacementSchema.optional(),
});

const UpdateComponentActionSchema = z.object({
  kind: z.literal("update_component"),
  id: z.string().uuid(),
  props: z.record(z.unknown()).optional(),
  mode: ComponentModeSchema.optional(),
});

const RemoveComponentActionSchema = z.object({
  kind: z.literal("remove_component"),
  id: z.string().uuid(),
});

const ReorderComponentsActionSchema = z.object({
  kind: z.literal("reorder_components"),
  container: PlacementSchema.optional(),
  order: z.array(z.string().uuid()),
});

export const DashboardActionSchema = z.discriminatedUnion("kind", [
  AddComponentActionSchema,
  UpdateComponentActionSchema,
  RemoveComponentActionSchema,
  ReorderComponentsActionSchema,
]);

export type DashboardAction = z.infer<typeof DashboardActionSchema>;

const removeFromLayout = (
  layout: LayoutTemplate,
  id: string,
): LayoutTemplate => {
  switch (layout.kind) {
    case "single":
      return {
        ...layout,
        components: layout.components.filter((c) => c !== id),
      };
    case "two_column":
      return {
        ...layout,
        left: layout.left.filter((c) => c !== id),
        right: layout.right.filter((c) => c !== id),
      };
    case "grid":
      return { ...layout, order: layout.order.filter((c) => c !== id) };
    case "tab_set":
      return {
        ...layout,
        tabs: layout.tabs.map((tab) => ({
          ...tab,
          components: tab.components.filter((c) => c !== id),
        })),
      };
    case "report":
      return {
        ...layout,
        sections: layout.sections.map((section) => ({
          ...section,
          components: section.components.filter((c) => c !== id),
        })),
      };
    default:
      return layout;
  }
};

const insertIntoLayout = (
  layout: LayoutTemplate,
  id: string,
  placement?: z.infer<typeof PlacementSchema>,
): LayoutTemplate => {
  const index = placement?.index ?? undefined;
  switch (layout.kind) {
    case "single": {
      const components = [...layout.components];
      components.splice(index ?? components.length, 0, id);
      return { ...layout, components };
    }
    case "two_column": {
      const target = placement?.column === "right" ? "right" : "left";
      const column = [...layout[target]];
      column.splice(index ?? column.length, 0, id);
      return { ...layout, [target]: column } as LayoutTemplate;
    }
    case "grid": {
      const order = [...layout.order];
      order.splice(index ?? order.length, 0, id);
      return { ...layout, order };
    }
    case "tab_set": {
      const targetTab =
        layout.tabs.find((tab) => tab.id === placement?.tabId) ??
        layout.tabs[0];
      const components = [...targetTab.components];
      components.splice(index ?? components.length, 0, id);
      return {
        ...layout,
        tabs: layout.tabs.map((tab) =>
          tab.id === targetTab.id ? { ...tab, components } : tab,
        ),
      };
    }
    case "report": {
      const targetSection =
        layout.sections.find(
          (section) => section.id === placement?.sectionId,
        ) ?? layout.sections[0];
      const components = [...targetSection.components];
      components.splice(index ?? components.length, 0, id);
      return {
        ...layout,
        sections: layout.sections.map((section) =>
          section.id === targetSection.id
            ? { ...section, components }
            : section,
        ),
      };
    }
    default:
      return layout;
  }
};

const reorderLayout = (
  layout: LayoutTemplate,
  order: string[],
  placement?: z.infer<typeof PlacementSchema>,
): LayoutTemplate => {
  switch (layout.kind) {
    case "single":
      return { ...layout, components: order };
    case "two_column": {
      const target = placement?.column === "right" ? "right" : "left";
      return { ...layout, [target]: order } as LayoutTemplate;
    }
    case "grid":
      return { ...layout, order };
    case "tab_set": {
      const targetTab =
        layout.tabs.find((tab) => tab.id === placement?.tabId) ??
        layout.tabs[0];
      return {
        ...layout,
        tabs: layout.tabs.map((tab) =>
          tab.id === targetTab.id ? { ...tab, components: order } : tab,
        ),
      };
    }
    case "report": {
      const targetSection =
        layout.sections.find(
          (section) => section.id === placement?.sectionId,
        ) ?? layout.sections[0];
      return {
        ...layout,
        sections: layout.sections.map((section) =>
          section.id === targetSection.id
            ? { ...section, components: order }
            : section,
        ),
      };
    }
    default:
      return layout;
  }
};

const replaceComponent = (
  components: ComponentInstance[],
  updated: ComponentInstance,
): ComponentInstance[] =>
  components.map((c) => (c.id === updated.id ? updated : c));

const removeComponent = (
  components: ComponentInstance[],
  id: string,
): ComponentInstance[] => components.filter((c) => c.id !== id);

export const applyDashboardAction = (
  dashboard: DashboardDefinition,
  action: DashboardAction,
): DashboardDefinition => {
  const parsed = DashboardDefinitionSchema.parse(dashboard);

  switch (action.kind) {
    case "add_component": {
      if (parsed.components.some((c) => c.id === action.component.id)) {
        throw new Error(
          `component with id ${action.component.id} already exists`,
        );
      }
      const components = [...parsed.components, action.component];
      const layout = insertIntoLayout(
        parsed.layout,
        action.component.id,
        action.placement,
      );
      return { ...parsed, components, layout };
    }
    case "update_component": {
      const target = parsed.components.find((c) => c.id === action.id);
      if (!target) {
        throw new Error(`component ${action.id} not found`);
      }
      const updated: ComponentInstance = {
        ...target,
        props: { ...target.props, ...action.props },
        mode: action.mode ?? target.mode,
      };
      return {
        ...parsed,
        components: replaceComponent(parsed.components, updated),
      };
    }
    case "remove_component": {
      const components = removeComponent(parsed.components, action.id);
      const layout = removeFromLayout(parsed.layout, action.id);
      return { ...parsed, components, layout };
    }
    case "reorder_components": {
      return {
        ...parsed,
        layout: reorderLayout(parsed.layout, action.order, action.container),
      };
    }
    default:
      return parsed;
  }
};
