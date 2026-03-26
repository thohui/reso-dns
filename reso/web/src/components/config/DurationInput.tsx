import { Field, HStack, Input, NativeSelect } from '@chakra-ui/react';
import { useCallback, useMemo } from 'react';

const UNITS = [
	{ label: 'milliseconds', value: 0.001 },
	{ label: 'seconds', value: 1 },
	{ label: 'minutes', value: 60 },
	{ label: 'hours', value: 3600 },
	{ label: 'days', value: 86400 },
] as const;

type Unit = (typeof UNITS)[number]['value'];
type AllowedUnit = (typeof UNITS)[number]['label'];

function decompose(
	totalSeconds: number,
	allowed: (typeof UNITS)[number][],
): { amount: number; unit: Unit } {
	for (let i = allowed.length - 1; i >= 0; i--) {
		const u = allowed[i];
		if (
			totalSeconds >= u.value &&
			Math.round(totalSeconds / u.value) * u.value === totalSeconds
		) {
			return { amount: totalSeconds / u.value, unit: u.value };
		}
	}
	// Doesn't divide evenly — fall back to smallest allowed unit
	const fallback = allowed[0];
	return {
		amount: Math.round(totalSeconds / fallback.value),
		unit: fallback.value,
	};
}

interface Props {
	value: number;
	allowedUnits?: AllowedUnit[];
	onChange: (value: number) => void;
	min?: number;
	conversion?: AllowedUnit;
	invalid?: boolean;
	errorText?: string;
}

const defaultAllowedUnits = UNITS.map((u) => u.label).filter(
	(label) => label !== 'seconds' && label !== 'milliseconds',
) as AllowedUnit[];

export function DurationInput({
	value,
	onChange,
	min = 60,
	allowedUnits = defaultAllowedUnits,
	conversion = 'seconds',
	invalid,
	errorText,
}: Props) {
	const conversionFactor = UNITS.find((u) => u.label === conversion)!.value;
	const allowedUnitsFull = UNITS.filter((u) => allowedUnits.includes(u.label));
	const totalSeconds = value * conversionFactor;
	const { amount, unit } = useMemo(
		() => decompose(totalSeconds, allowedUnitsFull),
		[totalSeconds, allowedUnitsFull],
	);

	const handleAmountChange = useCallback(
		(e: React.ChangeEvent<HTMLInputElement>) => {
			const n = Number.parseInt(e.target.value, 10);
			if (!Number.isNaN(n) && n > 0) {
				const seconds = Math.max(n * unit, min * conversionFactor);
				onChange(seconds / conversionFactor);
			}
		},
		[unit, min, conversionFactor, onChange],
	);

	const handleUnitChange = useCallback(
		(e: React.ChangeEvent<HTMLSelectElement>) => {
			const newUnit = Number(e.target.value) as Unit;
			const seconds = Math.max(amount * newUnit, min * conversionFactor);
			onChange(seconds / conversionFactor);
		},
		[amount, min, conversionFactor, onChange],
	);

	return (
		<Field.Root invalid={invalid}>
			<HStack gap='2'>
				<Input
					type='number'
					min={1}
					step={1}
					value={amount}
					onChange={handleAmountChange}
					flex='1'
				/>
				<NativeSelect.Root w='auto' minW='110px'>
					<NativeSelect.Field
						bg='bg.input'
						borderColor='border.input'
						value={unit}
						onChange={handleUnitChange}
					>
						{allowedUnitsFull.map((u) => (
							<option key={u.value} value={u.value}>
								{u.label}
							</option>
						))}
					</NativeSelect.Field>
					<NativeSelect.Indicator />
				</NativeSelect.Root>
			</HStack>
			{errorText && (
				<Field.ErrorText color='status.error'>{errorText}</Field.ErrorText>
			)}
		</Field.Root>
	);
}
