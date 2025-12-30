import { zodToJsonSchema } from "zod-to-json-schema";

import { DashboardActionSchema } from "../engine/dashboardActions";
import { defaultRegistry } from "../engine/ComponentRegistry";
import {
  BindingExamples,
  DashboardDefinitionSchema,
  LayoutTemplateSchema,
} from "./dashboard";

export const dashboardDefinitionJsonSchema = zodToJsonSchema(
  DashboardDefinitionSchema,
  { name: "DashboardDefinition" },
);

export const dashboardActionJsonSchema = zodToJsonSchema(
  DashboardActionSchema,
  { name: "DashboardAction" },
);

export const layoutTemplateJsonSchema = zodToJsonSchema(LayoutTemplateSchema, {
  name: "LayoutTemplate",
});

export const componentPropJsonSchemas = () =>
  defaultRegistry.list().map((registration) => ({
    type: registration.type,
    description: registration.description,
    allowedModes: registration.allowedModes,
    exampleProps: registration.exampleProps,
    schema: zodToJsonSchema(registration.propsSchema, {
      name: `${registration.type}Props`,
    }),
  }));

export const openAIFunctionSchemas = () => {
  const components = componentPropJsonSchemas();
  return {
    dashboardDefinition: {
      name: "create_dashboard_definition",
      description:
        "Create a dashboard definition with layout, component instances, and bindings.",
      parameters: dashboardDefinitionJsonSchema,
    },
    dashboardAction: {
      name: "mutate_dashboard",
      description: "Apply a dashboard mutation (add/update/remove/reorder).",
      parameters: dashboardActionJsonSchema,
    },
    components: components.map((component) => ({
      name: `component_${component.type}`,
      description: component.description,
      parameters: component.schema,
    })),
  };
};

export const toLLMContext = () => ({
  bindingSources: ["market", "portfolio", "statements", "scenarios"],
  layouts: ["single", "two_column", "grid", "tab_set", "report"],
  components: defaultRegistry.list().map((entry) => ({
    type: entry.type,
    description: entry.description,
    exampleProps: entry.exampleProps,
    allowedModes: entry.allowedModes,
  })),
  bindingExamples: BindingExamples,
});
