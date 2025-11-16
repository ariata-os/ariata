<script lang="ts">
	import { Page } from "$lib";
	import "iconify-icon";
	import { onMount } from "svelte";

	let loading = true;
	let saving = false;
	let saveSuccess = false;

	// Profile fields
	let fullName = "";
	let preferredName = "";
	let birthDate = "";
	let heightCm = "";
	let weightKg = "";
	let ethnicity = "";
	let homeStreet = "";
	let homeCity = "";
	let homeState = "";
	let homePostalCode = "";
	let homeCountry = "";
	let occupation = "";
	let employer = "";

	onMount(async () => {
		await loadProfile();
	});

	async function loadProfile() {
		loading = true;
		try {
			const response = await fetch("/api/profile");
			if (response.ok) {
				const profile = await response.json();

				// Populate fields from profile
				fullName = profile.full_name || "";
				preferredName = profile.preferred_name || "";
				birthDate = profile.birth_date || "";
				heightCm = profile.height_cm || "";
				weightKg = profile.weight_kg || "";
				ethnicity = profile.ethnicity || "";
				homeStreet = profile.home_street || "";
				homeCity = profile.home_city || "";
				homeState = profile.home_state || "";
				homePostalCode = profile.home_postal_code || "";
				homeCountry = profile.home_country || "";
				occupation = profile.occupation || "";
				employer = profile.employer || "";
			}
		} catch (error) {
			console.error("Failed to load profile:", error);
		} finally {
			loading = false;
		}
	}

	async function saveProfile() {
		saving = true;
		saveSuccess = false;
		try {
			const response = await fetch("/api/profile", {
				method: "PUT",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					full_name: fullName || null,
					preferred_name: preferredName || null,
					birth_date: birthDate || null,
					height_cm: heightCm ? parseFloat(heightCm) : null,
					weight_kg: weightKg ? parseFloat(weightKg) : null,
					ethnicity: ethnicity || null,
					home_street: homeStreet || null,
					home_city: homeCity || null,
					home_state: homeState || null,
					home_postal_code: homePostalCode || null,
					home_country: homeCountry || null,
					occupation: occupation || null,
					employer: employer || null,
				}),
			});

			if (response.ok) {
				saveSuccess = true;
				setTimeout(() => {
					saveSuccess = false;
				}, 3000);
			} else {
				throw new Error("Failed to save profile");
			}
		} catch (error) {
			console.error("Failed to save profile:", error);
			alert("Failed to save profile. Please try again.");
		} finally {
			saving = false;
		}
	}
</script>

<Page>
	<div class="max-w-3xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">
				Profile
			</h1>
			<p class="text-neutral-600">
				Manage your personal information and preferences
			</p>
		</div>

		{#if loading}
			<div class="text-center py-12 text-neutral-500">
				Loading profile...
			</div>
		{:else}
			<form on:submit|preventDefault={saveProfile} class="space-y-6">
				<!-- Identity Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Identity
					</h2>
					<div class="space-y-4">
						<div>
							<label
								for="fullName"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Full Name
							</label>
							<input
								type="text"
								id="fullName"
								bind:value={fullName}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="Your full legal name"
							/>
						</div>

						<div>
							<label
								for="preferredName"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Preferred Name
							</label>
							<input
								type="text"
								id="preferredName"
								bind:value={preferredName}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="How should the assistant address you?"
							/>
							<p class="text-xs text-neutral-500 mt-1">
								This will be used in conversations if set,
								otherwise your full name will be used
							</p>
						</div>

						<div>
							<label
								for="birthDate"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Birth Date
							</label>
							<input
								type="date"
								id="birthDate"
								bind:value={birthDate}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							/>
						</div>
					</div>
				</div>

				<!-- Physical Details Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Physical Details
					</h2>
					<div class="space-y-4">
						<div class="grid grid-cols-2 gap-4">
							<div>
								<label
									for="heightCm"
									class="block text-sm font-medium text-neutral-700 mb-2"
								>
									Height (cm)
								</label>
								<input
									type="number"
									step="0.01"
									id="heightCm"
									bind:value={heightCm}
									class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
									placeholder="175.5"
								/>
							</div>

							<div>
								<label
									for="weightKg"
									class="block text-sm font-medium text-neutral-700 mb-2"
								>
									Weight (kg)
								</label>
								<input
									type="number"
									step="0.01"
									id="weightKg"
									bind:value={weightKg}
									class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
									placeholder="70.5"
								/>
							</div>
						</div>

						<div>
							<label
								for="ethnicity"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Ethnicity
							</label>
							<input
								type="text"
								id="ethnicity"
								bind:value={ethnicity}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="Optional"
							/>
						</div>
					</div>
				</div>

				<!-- Home Address Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Home Address
					</h2>
					<div class="space-y-4">
						<div>
							<label
								for="homeStreet"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Street Address
							</label>
							<input
								type="text"
								id="homeStreet"
								bind:value={homeStreet}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="123 Main St"
							/>
						</div>

						<div class="grid grid-cols-2 gap-4">
							<div>
								<label
									for="homeCity"
									class="block text-sm font-medium text-neutral-700 mb-2"
								>
									City
								</label>
								<input
									type="text"
									id="homeCity"
									bind:value={homeCity}
									class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
									placeholder="San Francisco"
								/>
							</div>

							<div>
								<label
									for="homeState"
									class="block text-sm font-medium text-neutral-700 mb-2"
								>
									State/Province
								</label>
								<input
									type="text"
									id="homeState"
									bind:value={homeState}
									class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
									placeholder="CA"
								/>
							</div>
						</div>

						<div class="grid grid-cols-2 gap-4">
							<div>
								<label
									for="homePostalCode"
									class="block text-sm font-medium text-neutral-700 mb-2"
								>
									Postal Code
								</label>
								<input
									type="text"
									id="homePostalCode"
									bind:value={homePostalCode}
									class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
									placeholder="94102"
								/>
							</div>

							<div>
								<label
									for="homeCountry"
									class="block text-sm font-medium text-neutral-700 mb-2"
								>
									Country
								</label>
								<input
									type="text"
									id="homeCountry"
									bind:value={homeCountry}
									class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
									placeholder="United States"
								/>
							</div>
						</div>
					</div>
				</div>

				<!-- Work Information Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Work Information
					</h2>
					<div class="space-y-4">
						<div>
							<label
								for="occupation"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Occupation
							</label>
							<input
								type="text"
								id="occupation"
								bind:value={occupation}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="Software Engineer, Designer, Student, etc."
							/>
						</div>

						<div>
							<label
								for="employer"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Employer
							</label>
							<input
								type="text"
								id="employer"
								bind:value={employer}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="Company name (optional)"
							/>
							<p class="text-xs text-neutral-500 mt-1">
								Leave blank if self-employed or not applicable
							</p>
						</div>
					</div>
				</div>

				<!-- Save Button -->
				<div class="flex items-center gap-3 pt-2">
					<button
						type="submit"
						disabled={saving}
						class="px-6 py-2 bg-neutral-900 text-white rounded-md hover:bg-neutral-800 focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{saving ? "Saving..." : "Save Profile"}
					</button>
					{#if saveSuccess}
						<span
							class="text-sm text-green-600 flex items-center gap-1"
						>
							<iconify-icon icon="mdi:check-circle" width="16"
							></iconify-icon>
							Saved successfully
						</span>
					{/if}
				</div>
			</form>
		{/if}
	</div>
</Page>
